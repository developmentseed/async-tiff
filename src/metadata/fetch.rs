use std::ops::Range;

use bytes::{Bytes, BytesMut};
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
    fn fetch(&mut self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>>;
}

impl<T: AsyncFileReader> MetadataFetch for T {
    fn fetch(&mut self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        self.get_bytes(range)
    }
}

/// Buffering for the first `N` bytes of a file.
///
/// This is designed so that the async requests made by the underlying tag reader get intercepted
/// here and served from the existing buffer when possible.
#[derive(Debug)]
pub struct PrefetchBuffer<'a, F: MetadataFetch + Send + Sync> {
    fetch: &'a mut F,
    /// Invariant: buffers are monotonically increasing buffers starting at the beginning of the
    /// file
    buffers: Vec<Bytes>,
    /// The exponent used for deciding how much more data to fetch on overflow of the existing buffer.
    ///
    /// buffer_length ^ fetch_exponent
    overflow_fetch_exponent: f64,
}

impl<'a, F: MetadataFetch + Send + Sync> PrefetchBuffer<'a, F> {
    /// Construct a new PrefetchBuffer, catching the first `prefetch` bytes of the file.
    pub async fn new(
        fetch: &'a mut F,
        prefetch: u64,
        overflow_fetch_exponent: f64,
    ) -> AsyncTiffResult<Self> {
        let buffer = fetch.fetch(0..prefetch).await?;
        Ok(Self {
            fetch,
            buffers: vec![buffer],
            overflow_fetch_exponent,
        })
    }

    /// Expand the length of buffers that have been pre-fetched
    ///
    /// Returns the desired range and adds it to the cached buffers.
    async fn expand_prefetch(&mut self, range: Range<u64>) -> AsyncTiffResult<Bytes> {
        let existing_buffer_length = self.buffer_length() as u64;
        let additional_fetch =
            (existing_buffer_length as f64).powf(self.overflow_fetch_exponent) as u64;

        // Make sure that we fetch at least the entire desired range
        let new_range =
            existing_buffer_length..range.end.max(existing_buffer_length + additional_fetch);
        let buffer = self.fetch.fetch(new_range).await?;
        self.buffers.push(buffer);

        // Now extract the desired slice range
        Ok(self.buffer_slice(range))
    }

    /// The length of all cached buffers
    fn buffer_length(&self) -> usize {
        self.buffers.iter().fold(0, |acc, x| acc + x.len())
    }

    /// Access the buffer range out of the cached buffers
    ///
    /// ## Panics
    ///
    /// If the range does not fall completely within the pre-cached buffers.
    fn buffer_slice(&self, range: Range<u64>) -> Bytes {
        // Slices of the underlying cached buffers
        let mut output_buffers: Vec<Bytes> = vec![];

        // A counter that describes the global start of the currently-iterated `buf`
        let mut global_byte_offset: u64 = 0;

        for buf in self.buffers.iter() {
            // Subtract off the global_byte_offset and then see if it overlaps the current buffer
            let local_range =
                range.start.saturating_sub(global_byte_offset)..range.end - global_byte_offset;

            if ranges_overlap(&local_range, &(0..buf.len() as u64)) {
                let start = local_range.start as usize;
                let end = (local_range.end as usize).min(buf.len());
                output_buffers.push(buf.slice(start..end));
            }

            global_byte_offset += buf.len() as u64;
        }

        if output_buffers.len() == 1 {
            output_buffers.into_iter().next().unwrap()
        } else {
            let mut result = BytesMut::with_capacity(range.end as usize - range.start as usize);
            for output_buf in output_buffers.into_iter() {
                result.extend_from_slice(&output_buf);
            }
            result.freeze()
        }
    }
}

impl<F: MetadataFetch + Send + Sync> MetadataFetch for PrefetchBuffer<'_, F> {
    fn fetch(&mut self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        if range.end <= self.buffer_length() as _ {
            async { Ok(self.buffer_slice(range)) }.boxed()
        } else {
            self.expand_prefetch(range).boxed()
        }
    }
}

pub(crate) struct MetadataCursor<'a, F: MetadataFetch> {
    fetch: &'a mut F,
    offset: u64,
    endianness: Endianness,
}

impl<'a, F: MetadataFetch> MetadataCursor<'a, F> {
    pub fn new(fetch: &'a mut F, endianness: Endianness) -> Self {
        Self {
            fetch,
            offset: 0,
            endianness,
        }
    }

    pub fn new_with_offset(fetch: &'a mut F, endianness: Endianness, offset: u64) -> Self {
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

// https://stackoverflow.com/a/12888920
fn ranges_overlap(r1: &Range<u64>, r2: &Range<u64>) -> bool {
    r1.start.max(r2.start) <= r1.end.min(r2.end)
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Debug)]
    struct TestAsyncFileReader {
        buffer: Bytes,
    }

    impl TestAsyncFileReader {
        async fn get_range(&self, range: Range<u64>) -> AsyncTiffResult<Bytes> {
            assert!(range.start < self.buffer.len() as _);
            let end = range.end.min(self.buffer.len() as _);
            Ok(self.buffer.slice(range.start as usize..end as usize))
        }
    }

    impl MetadataFetch for TestAsyncFileReader {
        fn fetch(&mut self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
            self.get_range(range).boxed()
        }
    }

    #[tokio::test]
    async fn test_prefetch_overflow() {
        let underlying_buffer = b"abcdefghijklmno";
        let mut reader = TestAsyncFileReader {
            buffer: Bytes::from_static(underlying_buffer),
        };
        let mut prefetch_reader = PrefetchBuffer::new(&mut reader, 5, 1.).await.unwrap();

        // Cached
        assert_eq!(prefetch_reader.fetch(0..3).await.unwrap().as_ref(), b"abc");

        // Cached
        assert_eq!(
            prefetch_reader.fetch(0..5).await.unwrap().as_ref(),
            b"abcde"
        );

        // Expand fetch
        assert_eq!(
            prefetch_reader.fetch(0..10).await.unwrap().as_ref(),
            b"abcdefghij"
        );

        // Cached
        assert_eq!(
            prefetch_reader.fetch(0..10).await.unwrap().as_ref(),
            b"abcdefghij"
        );

        // Cached
        assert_eq!(
            prefetch_reader.fetch(0..15).await.unwrap().as_ref(),
            underlying_buffer
        );

        // Assert underlying buffers were cached
        assert_eq!(prefetch_reader.buffers[0].as_ref(), b"abcde");
        assert_eq!(prefetch_reader.buffers[1].as_ref(), b"fghij");
        assert_eq!(prefetch_reader.buffers[2].as_ref(), b"klmno");
    }
}
