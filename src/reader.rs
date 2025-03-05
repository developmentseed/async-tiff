//! Abstractions for network reading.

use std::fmt::Debug;
use std::io::Read;
use std::ops::Range;
use std::sync::Arc;

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use bytes::buf::Reader;
use bytes::{Buf, Bytes};
use futures::future::{BoxFuture, FutureExt, TryFutureExt};
use object_store::ObjectStore;

use crate::error::{AsyncTiffError, AsyncTiffResult};

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
pub trait AsyncFileReader: Debug + Send + Sync {
    /// Retrieve the bytes in `range`
    fn get_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>>;

    /// Retrieve multiple byte ranges. The default implementation will call `get_bytes`
    /// sequentially
    fn get_byte_ranges(
        &self,
        ranges: Vec<Range<u64>>,
    ) -> BoxFuture<'_, AsyncTiffResult<Vec<Bytes>>> {
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
    fn get_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        self.as_ref().get_bytes(range)
    }

    fn get_byte_ranges(
        &self,
        ranges: Vec<Range<u64>>,
    ) -> BoxFuture<'_, AsyncTiffResult<Vec<Bytes>>> {
        self.as_ref().get_byte_ranges(ranges)
    }
}

// #[cfg(feature = "tokio")]
// impl<T: tokio::io::AsyncRead + tokio::io::AsyncSeek + Unpin + Debug + Send + Sync> AsyncFileReader
//     for T
// {
//     fn get_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
//         use tokio::io::{AsyncReadExt, AsyncSeekExt};

//         async move {
//             self.seek(std::io::SeekFrom::Start(range.start)).await?;

//             let to_read = (range.end - range.start).try_into().unwrap();
//             let mut buffer = Vec::with_capacity(to_read);
//             let read = self.take(to_read as u64).read_to_end(&mut buffer).await?;
//             if read != to_read {
//                 return Err(AsyncTiffError::EndOfFile(to_read, read));
//             }

//             Ok(buffer.into())
//         }
//         .boxed()
//     }
// }

/// An AsyncFileReader that reads from an [`ObjectStore`] instance.
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
    fn get_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        let range = range.start as _..range.end as _;
        self.store
            .get_range(&self.path, range)
            .map_err(|e| e.into())
            .boxed()
    }

    fn get_byte_ranges(&self, ranges: Vec<Range<u64>>) -> BoxFuture<'_, AsyncTiffResult<Vec<Bytes>>>
    where
        Self: Send,
    {
        let ranges = ranges
            .into_iter()
            .map(|r| r.start as _..r.end as _)
            .collect::<Vec<_>>();
        async move {
            self.store
                .get_ranges(&self.path, &ranges)
                .await
                .map_err(|e| e.into())
        }
        .boxed()
    }
}

/// An AsyncFileReader that caches the first `prefetch` bytes of a file.
#[derive(Debug)]
pub struct PrefetchReader {
    reader: Box<dyn AsyncFileReader>,
    buffer: Bytes,
}

impl PrefetchReader {
    /// Construct a new PrefetchReader, catching the first `prefetch` bytes of the file.
    pub async fn new(reader: Box<dyn AsyncFileReader>, prefetch: u64) -> AsyncTiffResult<Self> {
        let buffer = reader.get_bytes(0..prefetch).await?;
        Ok(Self { reader, buffer })
    }
}

impl AsyncFileReader for PrefetchReader {
    fn get_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        if range.start < self.buffer.len() as _ {
            if range.end < self.buffer.len() as _ {
                let usize_range = range.start as usize..range.end as usize;
                let result = self.buffer.slice(usize_range);
                async { Ok(result) }.boxed()
            } else {
                // TODO: reuse partial internal buffer
                self.reader.get_bytes(range)
            }
        } else {
            self.reader.get_bytes(range)
        }
    }

    fn get_byte_ranges(&self, ranges: Vec<Range<u64>>) -> BoxFuture<'_, AsyncTiffResult<Vec<Bytes>>>
    where
        Self: Send,
    {
        // In practice, get_byte_ranges is only used for fetching tiles, which are unlikely to
        // overlap a metadata prefetch.
        self.reader.get_byte_ranges(ranges)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Endianness {
    LittleEndian,
    BigEndian,
}

/// A wrapper around an [ObjectStore] that provides a seek-oriented interface
// TODO: in the future add buffering to this
#[derive(Debug)]
pub(crate) struct AsyncCursor {
    reader: Box<dyn AsyncFileReader>,
    offset: u64,
    endianness: Endianness,
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
    pub(crate) async fn try_open_tiff(reader: Box<dyn AsyncFileReader>) -> AsyncTiffResult<Self> {
        // Initialize with little endianness and then set later
        let mut cursor = Self::new(reader, Endianness::LittleEndian);
        let magic_bytes = cursor.read(2).await?;
        let magic_bytes = magic_bytes.as_ref();

        // Should be b"II" for little endian or b"MM" for big endian
        if magic_bytes == Bytes::from_static(b"II") {
            cursor.endianness = Endianness::LittleEndian;
        } else if magic_bytes == Bytes::from_static(b"MM") {
            cursor.endianness = Endianness::BigEndian;
        } else {
            return Err(AsyncTiffError::General(format!(
                "unexpected magic bytes {magic_bytes:?}"
            )));
        };

        Ok(cursor)
    }

    /// Consume self and return the underlying [`AsyncFileReader`].
    #[allow(dead_code)]
    pub(crate) fn into_inner(self) -> Box<dyn AsyncFileReader> {
        self.reader
    }

    /// Read the given number of bytes, advancing the internal cursor state by the same amount.
    pub(crate) async fn read(&mut self, length: u64) -> AsyncTiffResult<EndianAwareReader> {
        let range = self.offset as _..(self.offset + length) as _;
        self.offset += length;
        let bytes = self.reader.get_bytes(range).await?;
        Ok(EndianAwareReader {
            reader: bytes.reader(),
            endianness: self.endianness,
        })
    }

    /// Read a u8 from the cursor, advancing the internal state by 1 byte.
    pub(crate) async fn read_u8(&mut self) -> AsyncTiffResult<u8> {
        self.read(1).await?.read_u8()
    }

    /// Read a i8 from the cursor, advancing the internal state by 1 byte.
    pub(crate) async fn read_i8(&mut self) -> AsyncTiffResult<i8> {
        self.read(1).await?.read_i8()
    }

    /// Read a u16 from the cursor, advancing the internal state by 2 bytes.
    pub(crate) async fn read_u16(&mut self) -> AsyncTiffResult<u16> {
        self.read(2).await?.read_u16()
    }

    /// Read a i16 from the cursor, advancing the internal state by 2 bytes.
    pub(crate) async fn read_i16(&mut self) -> AsyncTiffResult<i16> {
        self.read(2).await?.read_i16()
    }

    /// Read a u32 from the cursor, advancing the internal state by 4 bytes.
    pub(crate) async fn read_u32(&mut self) -> AsyncTiffResult<u32> {
        self.read(4).await?.read_u32()
    }

    /// Read a i32 from the cursor, advancing the internal state by 4 bytes.
    pub(crate) async fn read_i32(&mut self) -> AsyncTiffResult<i32> {
        self.read(4).await?.read_i32()
    }

    /// Read a u64 from the cursor, advancing the internal state by 8 bytes.
    pub(crate) async fn read_u64(&mut self) -> AsyncTiffResult<u64> {
        self.read(8).await?.read_u64()
    }

    /// Read a i64 from the cursor, advancing the internal state by 8 bytes.
    pub(crate) async fn read_i64(&mut self) -> AsyncTiffResult<i64> {
        self.read(8).await?.read_i64()
    }

    pub(crate) async fn read_f32(&mut self) -> AsyncTiffResult<f32> {
        self.read(4).await?.read_f32()
    }

    pub(crate) async fn read_f64(&mut self) -> AsyncTiffResult<f64> {
        self.read(8).await?.read_f64()
    }

    #[allow(dead_code)]
    pub(crate) fn reader(&self) -> &dyn AsyncFileReader {
        &self.reader
    }

    #[allow(dead_code)]
    pub(crate) fn endianness(&self) -> Endianness {
        self.endianness
    }

    /// Advance cursor position by a set amount
    pub(crate) fn advance(&mut self, amount: u64) {
        self.offset += amount;
    }

    pub(crate) fn seek(&mut self, offset: u64) {
        self.offset = offset;
    }

    pub(crate) fn position(&self) -> u64 {
        self.offset
    }
}

pub(crate) struct EndianAwareReader {
    reader: Reader<Bytes>,
    endianness: Endianness,
}

impl EndianAwareReader {
    /// Read a u8 from the cursor, advancing the internal state by 1 byte.
    pub(crate) fn read_u8(&mut self) -> AsyncTiffResult<u8> {
        Ok(self.reader.read_u8()?)
    }

    /// Read a i8 from the cursor, advancing the internal state by 1 byte.
    pub(crate) fn read_i8(&mut self) -> AsyncTiffResult<i8> {
        Ok(self.reader.read_i8()?)
    }

    pub(crate) fn read_u16(&mut self) -> AsyncTiffResult<u16> {
        match self.endianness {
            Endianness::LittleEndian => Ok(self.reader.read_u16::<LittleEndian>()?),
            Endianness::BigEndian => Ok(self.reader.read_u16::<BigEndian>()?),
        }
    }

    pub(crate) fn read_i16(&mut self) -> AsyncTiffResult<i16> {
        match self.endianness {
            Endianness::LittleEndian => Ok(self.reader.read_i16::<LittleEndian>()?),
            Endianness::BigEndian => Ok(self.reader.read_i16::<BigEndian>()?),
        }
    }

    pub(crate) fn read_u32(&mut self) -> AsyncTiffResult<u32> {
        match self.endianness {
            Endianness::LittleEndian => Ok(self.reader.read_u32::<LittleEndian>()?),
            Endianness::BigEndian => Ok(self.reader.read_u32::<BigEndian>()?),
        }
    }

    pub(crate) fn read_i32(&mut self) -> AsyncTiffResult<i32> {
        match self.endianness {
            Endianness::LittleEndian => Ok(self.reader.read_i32::<LittleEndian>()?),
            Endianness::BigEndian => Ok(self.reader.read_i32::<BigEndian>()?),
        }
    }

    pub(crate) fn read_u64(&mut self) -> AsyncTiffResult<u64> {
        match self.endianness {
            Endianness::LittleEndian => Ok(self.reader.read_u64::<LittleEndian>()?),
            Endianness::BigEndian => Ok(self.reader.read_u64::<BigEndian>()?),
        }
    }

    pub(crate) fn read_i64(&mut self) -> AsyncTiffResult<i64> {
        match self.endianness {
            Endianness::LittleEndian => Ok(self.reader.read_i64::<LittleEndian>()?),
            Endianness::BigEndian => Ok(self.reader.read_i64::<BigEndian>()?),
        }
    }

    pub(crate) fn read_f32(&mut self) -> AsyncTiffResult<f32> {
        match self.endianness {
            Endianness::LittleEndian => Ok(self.reader.read_f32::<LittleEndian>()?),
            Endianness::BigEndian => Ok(self.reader.read_f32::<BigEndian>()?),
        }
    }

    pub(crate) fn read_f64(&mut self) -> AsyncTiffResult<f64> {
        match self.endianness {
            Endianness::LittleEndian => Ok(self.reader.read_f64::<LittleEndian>()?),
            Endianness::BigEndian => Ok(self.reader.read_f64::<BigEndian>()?),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn into_inner(self) -> (Reader<Bytes>, Endianness) {
        (self.reader, self.endianness)
    }
}

impl AsRef<[u8]> for EndianAwareReader {
    fn as_ref(&self) -> &[u8] {
        self.reader.get_ref().as_ref()
    }
}

impl Read for EndianAwareReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(buf)
    }
}
