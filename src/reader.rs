//! Abstractions for network reading.

use std::fmt::Debug;
use std::io::Read;
use std::ops::Range;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use bytes::buf::Reader;
use bytes::{Buf, Bytes};
use futures::future::TryFutureExt;
#[cfg(feature = "object_store")]
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
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait AsyncFileReader: Debug + Send + Sync {
    /// Retrieve the bytes in `range`
    /// This function is called when reading in IFDs
    async fn get_bytes(&self, range: Range<u64>) -> AsyncTiffResult<Bytes>;

    /// Retrieve multiple byte ranges. The default implementation will call `get_bytes`
    /// sequentially
    /// This function is called when reading in IFDs
    async fn get_byte_ranges(&self, ranges: Vec<Range<u64>>) -> AsyncTiffResult<Vec<Bytes>> {
        let mut result = Vec::with_capacity(ranges.len());

        for range in ranges.into_iter() {
            let data = self.get_bytes(range).await?;
            result.push(data);
        }

        Ok(result)
    }

    /// Same as [`get_bytes`], but this function is called when retrieving
    /// compressed tile data
    async fn get_tile_bytes(&self, range: Range<u64>) -> AsyncTiffResult<Bytes> {
        self.get_bytes(range).await
    }

    /// Same as [`get_byte_ranges`], but this function is only called when retrieving
    /// compressed tile data
    async fn get_tile_byte_ranges(&self, ranges: Vec<Range<u64>>) -> AsyncTiffResult<Vec<Bytes>> {
        self.get_byte_ranges(ranges).await
    }
}

/// This allows Box<dyn AsyncFileReader + '_> to be used as an AsyncFileReader,
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AsyncFileReader for Box<dyn AsyncFileReader> {
    async fn get_bytes(&self, range: Range<u64>) -> AsyncTiffResult<Bytes> {
        self.get_bytes(range).await
    }

    async fn get_byte_ranges(&self, ranges: Vec<Range<u64>>) -> AsyncTiffResult<Vec<Bytes>> {
        self.get_byte_ranges(ranges).await
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
#[cfg(feature = "object_store")]
#[derive(Clone, Debug)]
pub struct ObjectReader {
    store: Arc<dyn ObjectStore>,
    path: object_store::path::Path,
}
#[cfg(feature = "object_store")]
impl ObjectReader {
    /// Creates a new [`ObjectReader`] for the provided [`ObjectStore`] and path
    ///
    /// [`ObjectMeta`] can be obtained using [`ObjectStore::list`] or [`ObjectStore::head`]
    pub fn new(store: Arc<dyn ObjectStore>, path: object_store::path::Path) -> Self {
        Self { store, path }
    }
}
#[cfg(feature = "object_store")]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AsyncFileReader for ObjectReader {
    async fn get_bytes(&self, range: Range<u64>) -> AsyncTiffResult<Bytes> {
        let range = range.start as _..range.end as _;
        self.store
            .get_range(&self.path, range)
            .map_err(|e| e.into())
            .await
    }

    async fn get_byte_ranges(&self, ranges: Vec<Range<u64>>) -> AsyncTiffResult<Vec<Bytes>>
    where
        Self: Send,
    {
        let ranges = ranges
            .into_iter()
            .map(|r| r.start as _..r.end as _)
            .collect::<Vec<_>>();
        self.store
            .get_ranges(&self.path, &ranges)
            .await
            .map_err(|e| e.into())
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
}
#[cfg(feature = "reqwest")]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AsyncFileReader for ReqwestReader {
    async fn get_bytes(&self, range: Range<u64>) -> AsyncTiffResult<Bytes> {
        // }
        // fn get_bytes<'async_trait>(&'async_trait self, range: Range<u64>) -> BoxFuture<'async_trait, AsyncTiffResult<Bytes>>
        // {
        let url = self.url.clone();
        let client = self.client.clone();
        // HTTP range is inclusive, so we need to subtract 1 from the end
        let range = format!("bytes={}-{}", range.start, range.end - 1);
        let response = client.get(url).header("Range", range).send().await?;
        let bytes = response.bytes().await?;
        Ok(bytes)
    }
}

/// An AsyncFileReader that caches the first `prefetch` bytes of a file.
#[derive(Debug)]
pub struct PrefetchReader {
    reader: Arc<dyn AsyncFileReader>,
    buffer: Bytes,
    cache: Arc<Mutex<(Range<u64>, Bytes)>>,
}

impl PrefetchReader {
    /// Construct a new PrefetchReader, catching the first `prefetch` bytes of the file.
    pub async fn new(reader: Arc<dyn AsyncFileReader>, prefetch: u64) -> AsyncTiffResult<Self> {
        let buffer = reader.get_bytes(0..prefetch).await?;
        let cache = Arc::new(Mutex::new((Range::default(), Bytes::new())));
        Ok(Self {
            reader,
            buffer,
            cache,
        })
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AsyncFileReader for PrefetchReader {
    async fn get_bytes(&self, range: Range<u64>) -> AsyncTiffResult<Bytes> {
        if range.start < self.buffer.len() as _ {
            if range.end < self.buffer.len() as _ {
                let usize_range = range.start as usize..range.end as usize;
                let result = self.buffer.slice(usize_range);
                Ok(result)
            } else {
                // TODO: reuse partial internal buffer
                self.reader.get_bytes(range).await
            }
        } else {
            // try to get from cache or re-initialize it
            {
                let lock = self.cache.lock().unwrap_or_else(|e| e.into_inner());
                if range.start >= lock.0.start
                    && range.start < lock.0.start
                    && range.end > lock.0.start
                    && range.end <= lock.0.end
                {
                    let usize_range =
                        (range.start - lock.0.start) as usize..(range.end - lock.0.start) as usize;
                    return Ok(lock.1.slice(usize_range));
                }
            }
            // aggressive COG caching based on the following heuristic:
            // If we are here, it probably means we're at a full-size image,
            // are done with the previous IFD(s), so we can evict the cache and
            // create a new one of size (assuming bigtiff):
            // TileOffsets is u64, TileByteCounts u32
            // so this IFD is
            // byte_count + byte_count/2 = 3/2*byte_count
            // then all further IFDs is
            // current + current/4 + current/16 + ... <= 4/3*current <- this is somewhat flawed
            // Above two lines simplify to
            // 4/3*(3/2byte_count) = 2*byte_count
            // In
            // https://isdasoil.s3.amazonaws.com/covariates/dem_30m/dem_30m.tif,
            // this falls just short because larger overviews have more tiles to
            // fill things up. In here we don't know how that would happen though
            // 272420 Long8 at offset 4270 -> 2179360 bytes (2MB) -> 4358720 total ->
            // 4362990 and last offset = 4367058, difference ~4kB
            let byte_count = range.end - range.start;
            let cache_range = range.start..range.start + 2 * byte_count;
            let cache = self.reader.get_bytes(cache_range.clone()).await?;
            {
                let mut lock = self.cache.lock().map_err(|_e| {
                    AsyncTiffError::General("Reader cache Mutex poisoned".to_string())
                })?;
                *lock = (cache_range, cache.clone());
            }
            Ok(cache.slice(0..byte_count as usize))
        }
    }

    async fn get_tile_bytes(&self, range: Range<u64>) -> AsyncTiffResult<Bytes> {
        self.reader.get_bytes(range).await
    }

    async fn get_byte_ranges(&self, ranges: Vec<Range<u64>>) -> AsyncTiffResult<Vec<Bytes>>
    where
        Self: Send,
    {
        // In practice, get_byte_ranges is only used for fetching tiles, which are unlikely to
        // overlap a metadata prefetch.
        self.reader.get_byte_ranges(ranges).await
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
    pub(crate) fn reader(&self) -> &Arc<dyn AsyncFileReader> {
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
