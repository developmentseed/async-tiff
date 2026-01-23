//! Caching strategies for metadata fetching.

use std::ops::Range;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use tokio::sync::Mutex;

use crate::error::AsyncTiffResult;
use crate::metadata::MetadataFetch;

/// Logic for managing a cache of sequential buffers
#[derive(Debug)]
struct SequentialBlockCache {
    /// Contiguous blocks from offset 0
    ///
    /// # Invariant
    /// - Buffers are contiguous from offset 0
    buffers: Vec<Bytes>,

    /// Total length cached (== sum of buffers lengths)
    len: u64,
}

impl SequentialBlockCache {
    /// Create a new, empty SequentialBlockCache
    fn new() -> Self {
        Self {
            buffers: vec![],
            len: 0,
        }
    }

    /// Check if the given range is fully contained within the cached buffers
    fn contains(&self, range: Range<u64>) -> bool {
        range.end <= self.len
    }

    /// Slice out the given range from the cached buffers
    fn slice(&self, range: Range<u64>) -> Bytes {
        // The size of the output buffer
        let out_len = (range.end - range.start) as usize;

        // The remaining range of bytes required. This range is updated as we traverse buffers, so
        // the indexes are relative to the current buffer.
        let mut remaining = range;
        let mut out_buffers: Vec<Bytes> = vec![];

        for buf in &self.buffers {
            let current_buf_len = buf.len() as u64;

            // this block falls entirely before the desired range start
            if remaining.start >= current_buf_len {
                remaining.start -= current_buf_len;
                remaining.end -= current_buf_len;
                continue;
            }

            // we slice bytes out of *this* block
            let start = remaining.start as usize;
            let length =
                (remaining.end - remaining.start).min(current_buf_len - remaining.start) as usize;
            let end = start + length;

            // nothing to take from this block
            if start == end {
                continue;
            }

            let chunk = buf.slice(start..end);
            out_buffers.push(chunk);

            // consumed some portion; update and potentially break
            remaining.start = 0;
            if remaining.end <= current_buf_len {
                break;
            }
            remaining.end -= current_buf_len;
        }

        if out_buffers.len() == 1 {
            out_buffers.into_iter().next().unwrap()
        } else {
            let mut out = BytesMut::with_capacity(out_len);
            for b in out_buffers {
                out.extend_from_slice(&b);
            }
            out.into()
        }
    }

    fn append_buffer(&mut self, buffer: Bytes) {
        self.len += buffer.len() as u64;
        self.buffers.push(buffer);
    }
}

/// A MetadataFetch implementation that caches fetched data in exponentially growing chunks,
/// sequentially from the beginning of the file.
#[derive(Debug)]
pub struct ReadaheadMetadataCache<F: MetadataFetch> {
    inner: F,
    cache: Arc<Mutex<SequentialBlockCache>>,
    initial: u64,
    multiplier: f64,
}

impl<F: MetadataFetch> ReadaheadMetadataCache<F> {
    /// Create a new ReadaheadMetadataCache wrapping the given MetadataFetch
    pub fn new(inner: F) -> Self {
        Self {
            inner,
            cache: Arc::new(Mutex::new(SequentialBlockCache::new())),
            initial: 32 * 1024,
            multiplier: 2.0,
        }
    }

    /// Access the inner MetadataFetch
    pub fn inner(&self) -> &F {
        &self.inner
    }

    /// Set the initial fetch size in bytes, otherwise defaults to 32 KiB
    pub fn with_initial_size(mut self, initial: u64) -> Self {
        self.initial = initial;
        self
    }

    /// Set the multiplier for subsequent fetch sizes, otherwise defaults to 2.0
    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }

    fn next_fetch_size(&self, existing_len: u64) -> u64 {
        if existing_len == 0 {
            self.initial
        } else {
            (existing_len as f64 * self.multiplier).round() as u64
        }
    }
}

#[async_trait]
impl<F: MetadataFetch + Send + Sync> MetadataFetch for ReadaheadMetadataCache<F> {
    async fn fetch(&self, range: Range<u64>) -> AsyncTiffResult<Bytes> {
        let mut cache = self.cache.lock().await;

        // First check if we already have the range cached
        if cache.contains(range.start..range.end) {
            return Ok(cache.slice(range));
        }

        // Compute the correct fetch range
        let start_len = cache.len;
        let needed = range.end.saturating_sub(start_len);
        let fetch_size = self.next_fetch_size(start_len).max(needed);
        let fetch_range = start_len..start_len + fetch_size;

        // Perform the fetch while holding mutex
        // (this is OK because the mutex is async)
        let bytes = self.inner.fetch(fetch_range).await?;

        // Now append safely
        cache.append_buffer(bytes);

        Ok(cache.slice(range))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Debug)]
    struct TestFetch {
        data: Bytes,
        /// The number of fetches that actually reach the raw Fetch implementation
        num_fetches: Arc<Mutex<u64>>,
    }

    impl TestFetch {
        fn new(data: Bytes) -> Self {
            Self {
                data,
                num_fetches: Arc::new(Mutex::new(0)),
            }
        }
    }

    #[async_trait]
    impl MetadataFetch for TestFetch {
        async fn fetch(&self, range: Range<u64>) -> crate::error::AsyncTiffResult<Bytes> {
            if range.start as usize >= self.data.len() {
                return Ok(Bytes::new());
            }

            let end = (range.end as usize).min(self.data.len());
            let slice = self.data.slice(range.start as _..end);
            let mut g = self.num_fetches.lock().await;
            *g += 1;
            Ok(slice)
        }
    }

    #[tokio::test]
    async fn test_readahead_cache() {
        let data = Bytes::from_static(b"abcdefghijklmnopqrstuvwxyz");
        let fetch = TestFetch::new(data.clone());
        let cache = ReadaheadMetadataCache::new(fetch)
            .with_initial_size(2)
            .with_multiplier(3.0);

        // Make initial request
        let result = cache.fetch(0..2).await.unwrap();
        assert_eq!(result.as_ref(), b"ab");
        assert_eq!(*cache.inner.num_fetches.lock().await, 1);

        // Making a request within the cached range should not trigger a new fetch
        let result = cache.fetch(1..2).await.unwrap();
        assert_eq!(result.as_ref(), b"b");
        assert_eq!(*cache.inner.num_fetches.lock().await, 1);

        // Making a request that exceeds the cached range should trigger a new fetch
        let result = cache.fetch(2..5).await.unwrap();
        assert_eq!(result.as_ref(), b"cde");
        assert_eq!(*cache.inner.num_fetches.lock().await, 2);

        // Multiplier should be accurate: initial was 2, next was 6 (2*3), so total cached is now 8
        let result = cache.fetch(5..8).await.unwrap();
        assert_eq!(result.as_ref(), b"fgh");
        assert_eq!(*cache.inner.num_fetches.lock().await, 2);

        // Should work even for fetch range larger than underlying buffer
        let result = cache.fetch(8..20).await.unwrap();
        assert_eq!(result.as_ref(), b"ijklmnopqrst");
        assert_eq!(*cache.inner.num_fetches.lock().await, 3);
    }

    #[test]
    fn test_sequential_block_cache_empty_buffers() {
        let mut cache = SequentialBlockCache::new();
        cache.append_buffer(Bytes::from_static(b"012"));
        cache.append_buffer(Bytes::from_static(b""));
        cache.append_buffer(Bytes::from_static(b"34"));
        cache.append_buffer(Bytes::from_static(b""));
        cache.append_buffer(Bytes::from_static(b"5"));
        cache.append_buffer(Bytes::from_static(b""));
        cache.append_buffer(Bytes::from_static(b"67"));

        // Range, does it exist, expected slice
        let test_cases = [
            (0..3, true, Bytes::from_static(b"012")),
            (4..7, true, Bytes::from_static(b"456")),
            (0..8, true, Bytes::from_static(b"01234567")),
            (6..6, true, Bytes::from_static(b"")),
            (6..9, false, Bytes::from_static(b"")),
            (9..9, false, Bytes::from_static(b"")),
            (8..10, false, Bytes::from_static(b"")),
        ];

        for (range, exists, expected) in test_cases {
            assert_eq!(cache.contains(range.clone()), exists);
            if exists {
                assert_eq!(cache.slice(range.clone()), expected);
            }
        }
    }
}
