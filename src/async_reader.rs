use std::io::{Cursor, SeekFrom};
use std::ops::Range;
use std::sync::Arc;

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use bytes::Bytes;
use futures::future::{BoxFuture, FutureExt, TryFutureExt};
use object_store::ObjectStore;

use crate::error::{AiocogeoError, Result};

/// The asynchronous interface used to read COG files
///
/// This was derived from the Parquet `AsyncFileReader`:
/// https://docs.rs/parquet/latest/parquet/arrow/async_reader/trait.AsyncFileReader.html
///
/// Notes:
///
/// 1. There is a default implementation for types that implement [`AsyncRead`]
///    and [`AsyncSeek`], for example [`tokio::fs::File`].
///
/// 2. [`ObjectReader`], available when the `object_store` crate feature
///    is enabled, implements this interface for [`ObjectStore`].
///
/// [`ObjectStore`]: object_store::ObjectStore
///
/// [`tokio::fs::File`]: https://docs.rs/tokio/latest/tokio/fs/struct.File.html
pub trait AsyncFileReader: Send {
    /// Retrieve the bytes in `range`
    fn get_bytes(&mut self, range: Range<usize>) -> BoxFuture<'_, Result<Bytes>>;

    /// Retrieve multiple byte ranges. The default implementation will call `get_bytes` sequentially
    fn get_byte_ranges(&mut self, ranges: Vec<Range<usize>>) -> BoxFuture<'_, Result<Vec<Bytes>>> {
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
    fn get_bytes(&mut self, range: Range<usize>) -> BoxFuture<'_, Result<Bytes>> {
        self.as_mut().get_bytes(range)
    }

    fn get_byte_ranges(&mut self, ranges: Vec<Range<usize>>) -> BoxFuture<'_, Result<Vec<Bytes>>> {
        self.as_mut().get_byte_ranges(ranges)
    }
}

#[cfg(feature = "tokio")]
impl<T: tokio::io::AsyncRead + tokio::io::AsyncSeek + Unpin + Send> AsyncFileReader for T {
    fn get_bytes(&mut self, range: Range<usize>) -> BoxFuture<'_, Result<Bytes>> {
        use tokio::io::{AsyncReadExt, AsyncSeekExt};

        async move {
            self.seek(SeekFrom::Start(range.start as u64)).await?;

            let to_read = range.end - range.start;
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
    fn get_bytes(&mut self, range: Range<usize>) -> BoxFuture<'_, Result<Bytes>> {
        self.store
            .get_range(&self.path, range)
            .map_err(|e| e.into())
            .boxed()
    }

    fn get_byte_ranges(&mut self, ranges: Vec<Range<usize>>) -> BoxFuture<'_, Result<Vec<Bytes>>>
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
        pub(crate) async fn $method_name(&mut self) -> $typ {
            let mut buf = Cursor::new(self.read(<$typ>::BITS as usize / 8).await);
            match self.endianness {
                Endianness::LittleEndian => buf.$method_name::<LittleEndian>().unwrap(),
                Endianness::BigEndian => buf.$method_name::<BigEndian>().unwrap(),
            }
        }
    };
}

impl AsyncCursor {
    pub(crate) fn new(reader: Box<dyn AsyncFileReader>) -> Self {
        Self {
            reader,
            offset: 0,
            endianness: Default::default(),
        }
    }

    pub(crate) fn set_endianness(&mut self, endianness: Endianness) {
        self.endianness = endianness;
    }

    pub(crate) fn into_inner(self) -> Box<dyn AsyncFileReader> {
        self.reader
    }

    pub(crate) async fn read(&mut self, length: usize) -> Bytes {
        let range = self.offset..self.offset + length;
        self.offset += length;
        self.reader.get_bytes(range).await.unwrap()
    }

    /// Read a u8 from the cursor
    pub(crate) async fn read_u8(&mut self) -> u8 {
        let buf = self.read(1).await;
        Cursor::new(buf).read_u8().unwrap()
    }

    /// Read a i8 from the cursor
    pub(crate) async fn read_i8(&mut self) -> i8 {
        let buf = self.read(1).await;
        Cursor::new(buf).read_i8().unwrap()
    }

    impl_read_byteorder!(read_u16, u16);
    impl_read_byteorder!(read_u32, u32);
    impl_read_byteorder!(read_u64, u64);
    impl_read_byteorder!(read_i16, i16);
    impl_read_byteorder!(read_i32, i32);
    impl_read_byteorder!(read_i64, i64);

    pub(crate) async fn read_f32(&mut self) -> f32 {
        let mut buf = Cursor::new(self.read(4).await);
        match self.endianness {
            Endianness::LittleEndian => buf.read_f32::<LittleEndian>().unwrap(),
            Endianness::BigEndian => buf.read_f32::<BigEndian>().unwrap(),
        }
    }

    pub(crate) async fn read_f64(&mut self) -> f64 {
        let mut buf = Cursor::new(self.read(8).await);
        match self.endianness {
            Endianness::LittleEndian => buf.read_f64::<LittleEndian>().unwrap(),
            Endianness::BigEndian => buf.read_f64::<BigEndian>().unwrap(),
        }
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
