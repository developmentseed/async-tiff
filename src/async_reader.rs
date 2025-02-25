use std::io::Cursor;
use std::ops::Range;
use std::sync::Arc;

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use bytes::Bytes;
use futures::future::{BoxFuture, FutureExt, TryFutureExt};
use object_store::ObjectStore;

use crate::error::{AiocogeoError, Result};

/// The asynchronous interface used to read COG files
///
/// This was derived from the Parquet
/// [`AsyncFileReader`](https://docs.rs/parquet/latest/parquet/arrow/async_reader/trait.AsyncFileReader.html)
///
/// Notes:
///
/// 1. There is a default implementation for types that implement [`tokio::io::AsyncRead`]
///    and [`tokio::io::AsyncSeek`], for example [`tokio::fs::File`].
///
/// 2. [`ObjectReader`], available when the `object_store` crate feature
///    is enabled, implements this interface for [`ObjectStore`].
///
/// [`ObjectStore`]: object_store::ObjectStore
///
/// [`tokio::fs::File`]: https://docs.rs/tokio/latest/tokio/fs/struct.File.html
pub trait AsyncFileReader: Send + Sync {
    /// Retrieve the bytes in `range`
    fn get_bytes(&mut self, range: Range<u64>) -> BoxFuture<'_, Result<Bytes>>;

    /// Retrieve multiple byte ranges. The default implementation will call `get_bytes` sequentially
    fn get_byte_ranges(&mut self, ranges: Vec<Range<u64>>) -> BoxFuture<'_, Result<Vec<Bytes>>> {
        async move {
            let mut result = Vec::with_capacity(ranges.len());

            for range in ranges.into_iter() {
                let data = self.get_bytes(range).await?;
                result.push(data);
            }

            Ok(result)
        }
        .boxed()
    }
}

/// This allows Box<dyn AsyncFileReader + '_> to be used as an AsyncFileReader,
impl AsyncFileReader for Box<dyn AsyncFileReader + '_> {
    fn get_bytes(&mut self, range: Range<u64>) -> BoxFuture<'_, Result<Bytes>> {
        self.as_mut().get_bytes(range)
    }

    fn get_byte_ranges(&mut self, ranges: Vec<Range<u64>>) -> BoxFuture<'_, Result<Vec<Bytes>>> {
        self.as_mut().get_byte_ranges(ranges)
    }
}

#[cfg(feature = "tokio")]
impl<T: tokio::io::AsyncRead + tokio::io::AsyncSeek + Unpin + Send + Sync> AsyncFileReader for T {
    fn get_bytes(&mut self, range: Range<u64>) -> BoxFuture<'_, Result<Bytes>> {
        use tokio::io::{AsyncReadExt, AsyncSeekExt};

        async move {
            self.seek(std::io::SeekFrom::Start(range.start)).await?;

            let to_read = (range.end - range.start).try_into().unwrap();
            let mut buffer = Vec::with_capacity(to_read);
            let read = self.take(to_read as u64).read_to_end(&mut buffer).await?;
            if read != to_read {
                return Err(AiocogeoError::EndOfFile(to_read, read));
            }

            Ok(buffer.into())
        }
        .boxed()
    }
}

#[derive(Clone, Debug)]
pub struct ObjectReader {
    store: Arc<dyn ObjectStore>,
    path: object_store::path::Path,
}

impl ObjectReader {
    /// Creates a new [`ObjectReader`] for the provided [`ObjectStore`] and path
    ///
    /// [`ObjectMeta`] can be obtained using [`ObjectStore::list`] or [`ObjectStore::head`]
    pub fn new(store: Arc<dyn ObjectStore>, path: object_store::path::Path) -> Self {
        Self { store, path }
    }
}

impl AsyncFileReader for ObjectReader {
    fn get_bytes(&mut self, range: Range<u64>) -> BoxFuture<'_, Result<Bytes>> {
        self.store
            .get_range(&self.path, range)
            .map_err(|e| e.into())
            .boxed()
    }

    fn get_byte_ranges(&mut self, ranges: Vec<Range<u64>>) -> BoxFuture<'_, Result<Vec<Bytes>>>
    where
        Self: Send,
    {
        async move {
            self.store
                .get_ranges(&self.path, &ranges)
                .await
                .map_err(|e| e.into())
        }
        .boxed()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Endianness {
    #[default]
    LittleEndian,
    BigEndian,
}

/// A wrapper around an [ObjectStore] that provides a seek-oriented interface
// TODO: in the future add buffering to this
pub(crate) struct AsyncCursor {
    reader: Box<dyn AsyncFileReader>,
    offset: usize,
    endianness: Endianness,
}

/// Macro to generate functions to read scalar values from the cursor
macro_rules! impl_read_byteorder {
    ($method_name:ident, $typ:ty) => {
        pub(crate) async fn $method_name(&mut self) -> Result<$typ> {
            let mut buf = Cursor::new(self.read(<$typ>::BITS as usize / 8).await?);
            match self.endianness {
                Endianness::LittleEndian => Ok(buf.$method_name::<LittleEndian>()?),
                Endianness::BigEndian => Ok(buf.$method_name::<BigEndian>()?),
            }
        }
    };
}

impl AsyncCursor {
    /// Create a new AsyncCursor from a reader and endianness.
    pub(crate) fn new(reader: Box<dyn AsyncFileReader>, endianness: Endianness) -> Self {
        Self {
            reader,
            offset: 0,
            endianness,
        }
    }

    /// Create a new AsyncCursor for a TIFF file, automatically inferring endianness from the first
    /// two bytes.
    pub(crate) async fn try_open_tiff(reader: Box<dyn AsyncFileReader>) -> Result<Self> {
        // Initialize with default endianness and then set later
        let mut cursor = Self::new(reader, Default::default());
        let magic_bytes = cursor.read(2).await?;

        // Should be b"II" for little endian or b"MM" for big endian
        if magic_bytes == Bytes::from_static(b"II") {
            cursor.endianness = Endianness::LittleEndian;
        } else if magic_bytes == Bytes::from_static(b"MM") {
            cursor.endianness = Endianness::BigEndian;
        } else {
            return Err(AiocogeoError::General(format!(
                "unexpected magic bytes {magic_bytes:?}"
            )));
        };

        Ok(cursor)
    }

    /// Consume self and return the underlying [`AsyncFileReader`].
    pub(crate) fn into_inner(self) -> Box<dyn AsyncFileReader> {
        self.reader
    }

    /// Read the given number of bytes, advancing the internal cursor state by the same amount.
    pub(crate) async fn read(&mut self, length: usize) -> Result<Bytes> {
        let range = self.offset as _..(self.offset + length) as _;
        self.offset += length;
        self.reader.get_bytes(range).await
    }

    /// Read a u8 from the cursor, advancing the internal state by 1 byte.
    pub(crate) async fn read_u8(&mut self) -> Result<u8> {
        let buf = self.read(1).await?;
        Ok(Cursor::new(buf).read_u8()?)
    }

    /// Read a i8 from the cursor, advancing the internal state by 1 byte.
    pub(crate) async fn read_i8(&mut self) -> Result<i8> {
        let buf = self.read(1).await?;
        Ok(Cursor::new(buf).read_i8()?)
    }

    impl_read_byteorder!(read_u16, u16);
    impl_read_byteorder!(read_u32, u32);
    impl_read_byteorder!(read_u64, u64);
    impl_read_byteorder!(read_i16, i16);
    impl_read_byteorder!(read_i32, i32);
    impl_read_byteorder!(read_i64, i64);

    pub(crate) async fn read_f32(&mut self) -> Result<f32> {
        let mut buf = Cursor::new(self.read(4).await?);
        let out = match self.endianness {
            Endianness::LittleEndian => buf.read_f32::<LittleEndian>()?,
            Endianness::BigEndian => buf.read_f32::<BigEndian>()?,
        };
        Ok(out)
    }

    pub(crate) async fn read_f64(&mut self) -> Result<f64> {
        let mut buf = Cursor::new(self.read(8).await?);
        let out = match self.endianness {
            Endianness::LittleEndian => buf.read_f64::<LittleEndian>()?,
            Endianness::BigEndian => buf.read_f64::<BigEndian>()?,
        };
        Ok(out)
    }

    #[allow(dead_code)]
    pub(crate) fn reader(&self) -> &dyn AsyncFileReader {
        &self.reader
    }

    /// Advance cursor position by a set amount
    pub(crate) fn advance(&mut self, amount: usize) {
        self.offset += amount;
    }

    pub(crate) fn seek(&mut self, offset: usize) {
        self.offset = offset;
    }

    pub(crate) fn position(&self) -> usize {
        self.offset
    }
}
