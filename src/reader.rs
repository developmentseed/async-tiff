//! Abstractions for network reading.

use std::fmt::Debug;
use std::io::Read;
use std::ops::Range;
use std::sync::Arc;

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use bytes::buf::Reader;
use bytes::{Buf, Bytes};
use futures::future::{BoxFuture, FutureExt};
use futures::TryFutureExt;

use crate::error::AsyncTiffResult;

/// The asynchronous interface used to read COG files
///
/// This was derived from the Parquet
/// [`AsyncFileReader`](https://docs.rs/parquet/latest/parquet/arrow/async_reader/trait.AsyncFileReader.html)
///
/// Notes:
///
/// 1. [`ObjectReader`], available when the `object_store` crate feature
///    is enabled, implements this interface for [`ObjectStore`].
///
/// 2. You can use [`TokioReader`] to implement [`AsyncFileReader`] for types that implement
///    [`tokio::io::AsyncRead`] and [`tokio::io::AsyncSeek`], for example [`tokio::fs::File`].
///
/// [`ObjectStore`]: object_store::ObjectStore
///
/// [`tokio::fs::File`]: https://docs.rs/tokio/latest/tokio/fs/struct.File.html
pub trait AsyncFileReader: Debug + Send + Sync {
    /// Retrieve the bytes in `range` as part of a request for image data, not header metadata.
    ///
    /// This is also used as the default implementation of [`MetadataFetch`] if not overridden.
    fn get_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>>;

    /// Retrieve multiple byte ranges as part of a request for image data, not header metadata. The
    /// default implementation will call `get_bytes` sequentially
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

/// This allows Arc<dyn AsyncFileReader + '_> to be used as an AsyncFileReader,
impl AsyncFileReader for Arc<dyn AsyncFileReader + '_> {
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

/// A wrapper for things that implement [AsyncRead] and [AsyncSeek] to also implement
/// [AsyncFileReader].
///
/// This wrapper is needed because `AsyncRead` and `AsyncSeek` require mutable access to seek and
/// read data, while the `AsyncFileReader` trait requires immutable access to read data.
///
/// This wrapper stores the inner reader in a `Mutex`.
///
/// [AsyncRead]: tokio::io::AsyncRead
/// [AsyncSeek]: tokio::io::AsyncSeek
#[cfg(feature = "tokio")]
#[derive(Debug)]
pub struct TokioReader<T: tokio::io::AsyncRead + tokio::io::AsyncSeek + Unpin + Send + Debug>(
    tokio::sync::Mutex<T>,
);

#[cfg(feature = "tokio")]
impl<T: tokio::io::AsyncRead + tokio::io::AsyncSeek + Unpin + Send + Debug> TokioReader<T> {
    /// Create a new TokioReader from a reader.
    pub fn new(inner: T) -> Self {
        Self(tokio::sync::Mutex::new(inner))
    }

    async fn make_range_request(&self, range: Range<u64>) -> AsyncTiffResult<Bytes> {
        use std::io::SeekFrom;
        use tokio::io::{AsyncReadExt, AsyncSeekExt};

        use crate::error::AsyncTiffError;

        let mut file = self.0.lock().await;

        file.seek(SeekFrom::Start(range.start)).await?;

        let to_read = range.end - range.start;
        let mut buffer = Vec::with_capacity(to_read as usize);
        let read = file.read(&mut buffer).await? as u64;
        if read != to_read {
            return Err(AsyncTiffError::EndOfFile(to_read, read));
        }

        Ok(buffer.into())
    }
}

#[cfg(feature = "tokio")]
impl<T: tokio::io::AsyncRead + tokio::io::AsyncSeek + Unpin + Send + Debug> AsyncFileReader
    for TokioReader<T>
{
    fn get_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        self.make_range_request(range).boxed()
    }
}

/// An AsyncFileReader that reads from an [`ObjectStore`] instance.
#[cfg(feature = "object_store")]
#[derive(Clone, Debug)]
pub struct ObjectReader {
    store: Arc<dyn object_store::ObjectStore>,
    path: object_store::path::Path,
}

#[cfg(feature = "object_store")]
impl ObjectReader {
    /// Creates a new [`ObjectReader`] for the provided [`ObjectStore`] and path
    ///
    /// [`ObjectMeta`] can be obtained using [`ObjectStore::list`] or [`ObjectStore::head`]
    pub fn new(store: Arc<dyn object_store::ObjectStore>, path: object_store::path::Path) -> Self {
        Self { store, path }
    }

    async fn make_range_request(&self, range: Range<u64>) -> AsyncTiffResult<Bytes> {
        let range = range.start as _..range.end as _;
        self.store
            .get_range(&self.path, range)
            .map_err(|e| e.into())
            .await
    }
}

#[cfg(feature = "object_store")]
impl AsyncFileReader for ObjectReader {
    fn get_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        self.make_range_request(range).boxed()
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

/// An AsyncFileReader that reads from a URL using reqwest.
#[cfg(feature = "reqwest")]
#[derive(Debug, Clone)]
pub struct ReqwestReader {
    client: reqwest::Client,
    url: reqwest::Url,
}

#[cfg(feature = "reqwest")]
impl ReqwestReader {
    /// Construct a new ReqwestReader from a reqwest client and URL.
    pub fn new(client: reqwest::Client, url: reqwest::Url) -> Self {
        Self { client, url }
    }

    fn make_range_request(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        let url = self.url.clone();
        let client = self.client.clone();
        // HTTP range is inclusive, so we need to subtract 1 from the end
        let range = format!("bytes={}-{}", range.start, range.end - 1);
        async move {
            let response = client
                .get(url)
                .header("Range", range)
                .send()
                .await?
                .error_for_status()?;
            let bytes = response.bytes().await?;
            Ok(bytes)
        }
        .boxed()
    }
}

#[cfg(feature = "reqwest")]
impl AsyncFileReader for ReqwestReader {
    fn get_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        self.make_range_request(range)
    }
}

/// Endianness
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    /// Little Endian
    LittleEndian,
    /// Big Endian
    BigEndian,
}

pub(crate) struct EndianAwareReader {
    reader: Reader<Bytes>,
    endianness: Endianness,
}

impl EndianAwareReader {
    pub(crate) fn new(bytes: Bytes, endianness: Endianness) -> Self {
        Self {
            reader: bytes.reader(),
            endianness,
        }
    }

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
