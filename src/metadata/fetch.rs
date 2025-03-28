use std::ops::Range;

use bytes::Bytes;
use futures::future::BoxFuture;
use futures::FutureExt;

use crate::error::AsyncTiffResult;
use crate::reader::{AsyncFileReader, EndianAwareReader, Endianness};

/// A data source that can be used with [`TiffMetadataReader`] and [`ImageFileDirectoryReader`] to
/// load [`ImageFileDirectory`]s.
///
/// Note that implementation is provided for [`AsyncFileReader`].
pub trait MetadataFetch {
    /// Return a future that fetches the specified range of bytes asynchronously
    ///
    /// Note the returned type is a boxed future, often created by
    /// [futures::FutureExt::boxed]. See the trait documentation for an example.
    fn fetch(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>>;
}

impl<T: AsyncFileReader> MetadataFetch for T {
    fn fetch(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        self.get_bytes(range)
    }
}

/// Buffering for the first `N` bytes of a file.
///
/// This is designed so that the async requests made by the underlying tag reader get intercepted
/// here and served from the existing buffer when possible.
#[derive(Debug)]
pub struct PrefetchBuffer<F: MetadataFetch> {
    fetch: F,
    buffer: Bytes,
}

impl<F: MetadataFetch> PrefetchBuffer<F> {
    /// Construct a new PrefetchBuffer, catching the first `prefetch` bytes of the file.
    pub async fn new(fetch: F, prefetch: u64) -> AsyncTiffResult<Self> {
        let buffer = fetch.fetch(0..prefetch).await?;
        Ok(Self { fetch, buffer })
    }
}

impl<F: MetadataFetch> MetadataFetch for PrefetchBuffer<F> {
    fn fetch(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        if range.start < self.buffer.len() as _ {
            if range.end < self.buffer.len() as _ {
                let usize_range = range.start as usize..range.end as usize;
                let result = self.buffer.slice(usize_range);
                async { Ok(result) }.boxed()
            } else {
                // TODO: reuse partial internal buffer
                self.fetch.fetch(range)
            }
        } else {
            self.fetch.fetch(range)
        }
    }
}

pub(crate) struct MetadataCursor<'a, F: MetadataFetch> {
    fetch: &'a F,
    offset: u64,
    endianness: Endianness,
}

impl<'a, F: MetadataFetch> MetadataCursor<'a, F> {
    pub fn new(fetch: &'a F, endianness: Endianness) -> Self {
        Self {
            fetch,
            offset: 0,
            endianness,
        }
    }

    pub fn new_with_offset(fetch: &'a F, endianness: Endianness, offset: u64) -> Self {
        Self {
            fetch,
            offset,
            endianness,
        }
    }

    pub fn with_offset(mut self, offset: u64) -> Self {
        self.offset = offset;
        self
    }

    pub fn seek(&mut self, offset: u64) {
        self.offset = offset;
    }

    /// Advance cursor position by a set amount
    pub(crate) fn advance(&mut self, amount: u64) {
        self.offset += amount;
    }

    /// Read the given number of bytes, advancing the internal cursor state by the same amount.
    pub(crate) async fn read(&mut self, length: u64) -> AsyncTiffResult<EndianAwareReader> {
        let range = self.offset as _..(self.offset + length) as _;
        self.offset += length;
        let bytes = self.fetch.fetch(range).await?;
        Ok(EndianAwareReader::new(bytes, self.endianness))
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
}
