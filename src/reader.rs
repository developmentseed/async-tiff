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

use crate::error::{AsyncTiffError, AsyncTiffResult};

#[cfg(all(not(feature = "tokio"), feature = "async_mutex"))]
use async_mutex::Mutex;
#[cfg(feature = "tokio")]
use tokio::sync::Mutex;

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
    /// Retrieve the bytes in `range` as part of a request for header metadata.
    fn get_metadata_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>>;

    /// Retrieve the bytes in `range` as part of a request for image data, not header metadata.
    fn get_image_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>>;

    /// Retrieve multiple byte ranges as part of a request for image data, not header metadata. The
    /// default implementation will call `get_image_bytes` sequentially
    fn get_image_byte_ranges(
        &self,
        ranges: Vec<Range<u64>>,
    ) -> BoxFuture<'_, AsyncTiffResult<Vec<Bytes>>> {
        async move {
            let mut result = Vec::with_capacity(ranges.len());

            for range in ranges.into_iter() {
                let data = self.get_image_bytes(range).await?;
                result.push(data);
            }

            Ok(result)
        }
        .boxed()
    }
}

/// This allows Box<dyn AsyncFileReader + '_> to be used as an AsyncFileReader,
impl AsyncFileReader for Box<dyn AsyncFileReader + '_> {
    fn get_metadata_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        self.as_ref().get_metadata_bytes(range)
    }

    fn get_image_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        self.as_ref().get_image_bytes(range)
    }

    fn get_image_byte_ranges(
        &self,
        ranges: Vec<Range<u64>>,
    ) -> BoxFuture<'_, AsyncTiffResult<Vec<Bytes>>> {
        self.as_ref().get_image_byte_ranges(ranges)
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
    fn get_metadata_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        self.make_range_request(range).boxed()
    }

    fn get_image_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
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
    fn get_metadata_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        self.make_range_request(range).boxed()
    }

    fn get_image_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        self.make_range_request(range).boxed()
    }

    fn get_image_byte_ranges(
        &self,
        ranges: Vec<Range<u64>>,
    ) -> BoxFuture<'_, AsyncTiffResult<Vec<Bytes>>>
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
            let response = client.get(url).header("Range", range).send().await?;
            let bytes = response.bytes().await?;
            Ok(bytes)
        }
        .boxed()
    }
}

#[cfg(feature = "reqwest")]
impl AsyncFileReader for ReqwestReader {
    fn get_metadata_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        self.make_range_request(range)
    }

    fn get_image_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        self.make_range_request(range)
    }
}

impl AsyncFileReader for Bytes {
    fn get_image_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        self.get_metadata_bytes(range)
    }

    fn get_metadata_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        if range.end <= self.len() as _ {
            let usize_range = range.start as usize..range.end as usize;
            async { Ok(self.slice(usize_range)) }.boxed()
        } else {
            let range_len = range.end - range.start;
            let range_read = range_len - (range.end - self.len() as u64);
            async move { Err(AsyncTiffError::EndOfFile(range_len, range_read)) }.boxed()
        }
    }
}

/// An AsyncFileReader that caches the first `prefetch` bytes of a file. Also
/// holds a cache for out-of-prefetch metadata.
///
/// When a request is out-of-bounds, an estimate of the remaining
/// `tile_offsets`+`tile_byte_counts` array is made as
/// `(2*len+4*len.isqrt()).max(prefetch)`, where `len` is the length of the
/// requested range and `prefetch` is the size of the prefetch buffer.
///
/// # Examples
///
/// ```
/// # futures::executor::block_on( async { // https://stackoverflow.com/a/64597248/14681457
/// # use async_tiff::error::AsyncTiffError;
/// # use async_tiff::reader::{AsyncFileReader, PrefetchReader};
/// # use bytes::Bytes;
/// # use std::sync::Arc;
/// #
/// # let file = Bytes::from_static(&[42u8; 128 * 1024]);
/// #
/// // create a reader that prefetches the first 16 kB
/// let reader = PrefetchReader::new(Arc::new(file), 16 * 1024).await?;
///
/// // get some data from the prefetch
/// assert_eq!(*reader.get_metadata_bytes(42..1312).await?, [42; 1312 - 42]);
///
/// const start: u64 = 16 * 1024;
/// const len: u64 = 16 * 1024;
/// // get some data from outside the prefetch range
/// assert_eq!(
///     *reader.get_metadata_bytes(start..start + len).await?,
///     [42; 16 * 1024]
/// );
///
/// // this is now also (exactly) cached
/// assert_eq!(
///     *reader
///         .get_metadata_bytes(start..start + 2 * (len + 2 * len.isqrt()))
///         .await?,
///     [42; 2 * (len + 2 * len.isqrt()) as usize]
/// );
///
/// // this will not check the cache
/// reader.get_image_bytes(start..start + len).await?;
///
/// # Ok::<(),AsyncTiffError>(())
/// # });
/// ```
#[cfg(any(feature = "tokio", feature = "async_mutex"))]
#[derive(Debug)]
pub struct PrefetchReader {
    reader: Arc<dyn AsyncFileReader>,
    buffer: Bytes,
    tile_info_cache: Mutex<(Range<u64>, Bytes)>,
}

#[cfg(any(feature = "tokio", feature = "async_mutex"))]
impl PrefetchReader {
    /// Construct a new PrefetchReader, catching the first `prefetch` bytes of the file.
    pub async fn new(reader: Arc<dyn AsyncFileReader>, prefetch: u64) -> AsyncTiffResult<Self> {
        let buffer = reader.get_metadata_bytes(0..prefetch).await?;
        let tile_info_cache = Mutex::new((0..0, Bytes::new()));
        Ok(Self {
            reader,
            buffer,
            tile_info_cache,
        })
    }
}

#[cfg(any(feature = "tokio", feature = "async_mutex"))]
impl AsyncFileReader for PrefetchReader {
    fn get_metadata_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        // check prefetch
        if range.end < self.buffer.len() as _ {
            let usize_range = range.start as usize..range.end as usize;
            let result = self.buffer.slice(usize_range);
            async { Ok(result) }.boxed()
        } else {
            async move {
                {
                    // check cache
                    let lock = self.tile_info_cache.lock().await;
                    if range.start >= lock.0.start && range.end <= lock.0.end {
                        let usize_range = (range.start - lock.0.start) as usize
                            ..(range.end - lock.0.start) as usize;
                        return Ok(lock.1.slice(usize_range));
                    }
                }
                // determine new cache size
                let range_len = range.end - range.start;
                // estimate (for bigtiff):
                // each overview is 1/4 the previous =geometric series=> 4/3*range_len
                // assume request was TileOffsets =Long8+Long=> 3/2*4/3*range_len = 2*range_len
                //
                // add edge of one overview down:
                // n_tiles0 = range_len/8
                // n_tiles1 = 1/4*n_tiles0
                // edge_tiles1 ≈ √n_tiles1 = √range_len/√2⁶ = √range_len/(4√2)
                // each edge is 1/2 the previous =geometric series=> 2*edge_tiles1 = √range_len/(2√2)
                // edge_bytes = 8*√range_len/(2√2) = 4/√2*√range_len
                // Long8+Long => 3/2*4/√2*√range_len = 6/√2*√range_len ≈ 4*√range_len
                //
                // 2*range_len+4*√range_len
                let estimate = (2 * range_len + 4 * range_len.isqrt()).max(self.buffer.len() as _);
                let new_c_range = range.start..range.start + estimate;

                // put in new cache
                let res = self.reader.get_metadata_bytes(new_c_range.clone()).await?;
                {
                    let mut lock = self.tile_info_cache.lock().await;
                    *lock = (new_c_range, res.clone());
                }

                // yay
                Ok(res.slice(0..range_len as _))
            }
            .boxed()
        }
    }

    fn get_image_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        // In practice, get_image_bytes is only used for fetching tiles, which are unlikely
        // to overlap a metadata prefetch.
        self.reader.get_image_bytes(range)
    }

    fn get_image_byte_ranges(
        &self,
        ranges: Vec<Range<u64>>,
    ) -> BoxFuture<'_, AsyncTiffResult<Vec<Bytes>>>
    where
        Self: Send,
    {
        // In practice, get_image_byte_ranges is only used for fetching tiles, which are unlikely
        // to overlap a metadata prefetch.
        self.reader.get_image_byte_ranges(ranges)
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
    reader: Arc<dyn AsyncFileReader>,
    offset: u64,
    endianness: Endianness,
}

impl AsyncCursor {
    /// Create a new AsyncCursor from a reader and endianness.
    pub(crate) fn new(reader: Arc<dyn AsyncFileReader>, endianness: Endianness) -> Self {
        Self {
            reader,
            offset: 0,
            endianness,
        }
    }

    /// Create a new AsyncCursor for a TIFF file, automatically inferring endianness from the first
    /// two bytes.
    pub(crate) async fn try_open_tiff(reader: Arc<dyn AsyncFileReader>) -> AsyncTiffResult<Self> {
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
    pub(crate) fn into_inner(self) -> Arc<dyn AsyncFileReader> {
        self.reader
    }

    /// Read the given number of bytes, advancing the internal cursor state by the same amount.
    pub(crate) async fn read(&mut self, length: u64) -> AsyncTiffResult<EndianAwareReader> {
        let range = self.offset as _..(self.offset + length) as _;
        self.offset += length;
        let bytes = self.reader.get_metadata_bytes(range).await?;
        Ok(EndianAwareReader {
            reader: bytes.reader(),
            endianness: self.endianness,
        })
    }

    /// Read a u8 from the cursor, advancing the internal state by 1 byte.
    #[allow(dead_code)]
    pub(crate) async fn read_u8(&mut self) -> AsyncTiffResult<u8> {
        self.read(1).await?.read_u8()
    }

    /// Read a i8 from the cursor, advancing the internal state by 1 byte.
    #[allow(dead_code)]
    pub(crate) async fn read_i8(&mut self) -> AsyncTiffResult<i8> {
        self.read(1).await?.read_i8()
    }

    /// Read a u16 from the cursor, advancing the internal state by 2 bytes.
    #[allow(dead_code)]
    pub(crate) async fn read_u16(&mut self) -> AsyncTiffResult<u16> {
        self.read(2).await?.read_u16()
    }

    /// Read a i16 from the cursor, advancing the internal state by 2 bytes.
    #[allow(dead_code)]
    pub(crate) async fn read_i16(&mut self) -> AsyncTiffResult<i16> {
        self.read(2).await?.read_i16()
    }

    /// Read a u32 from the cursor, advancing the internal state by 4 bytes.
    #[allow(dead_code)]
    pub(crate) async fn read_u32(&mut self) -> AsyncTiffResult<u32> {
        self.read(4).await?.read_u32()
    }

    /// Read a i32 from the cursor, advancing the internal state by 4 bytes.
    #[allow(dead_code)]
    pub(crate) async fn read_i32(&mut self) -> AsyncTiffResult<i32> {
        self.read(4).await?.read_i32()
    }

    /// Read a u64 from the cursor, advancing the internal state by 8 bytes.
    pub(crate) async fn read_u64(&mut self) -> AsyncTiffResult<u64> {
        self.read(8).await?.read_u64()
    }

    /// Read a i64 from the cursor, advancing the internal state by 8 bytes.
    #[allow(dead_code)]
    pub(crate) async fn read_i64(&mut self) -> AsyncTiffResult<i64> {
        self.read(8).await?.read_i64()
    }

    #[allow(dead_code)]
    pub(crate) async fn read_f32(&mut self) -> AsyncTiffResult<f32> {
        self.read(4).await?.read_f32()
    }

    #[allow(dead_code)]
    pub(crate) async fn read_f64(&mut self) -> AsyncTiffResult<f64> {
        self.read(8).await?.read_f64()
    }

    pub(crate) fn reader(&self) -> &Arc<dyn AsyncFileReader> {
        &self.reader
    }

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

    #[allow(dead_code)]
    pub(crate) fn position(&self) -> u64 {
        self.offset
    }
}

pub(crate) struct EndianAwareReader {
    reader: Reader<Bytes>,
    endianness: Endianness,
}

impl EndianAwareReader {
    pub(crate) fn new(reader: Reader<Bytes>, endianness: Endianness) -> Self {
        Self { reader, endianness }
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
