//! Caching strategies for metadata fetching.

use std::ops::Range;
use std::sync::Arc;

use bytes::{Bytes, BytesMut};
use futures::future::BoxFuture;
use tokio::sync::Mutex;

use crate::error::AsyncTiffResult;
use crate::metadata::MetadataFetch;

/// Logic for managing a cache of sequential buffers
struct SequentialCache {
    /// Contiguous blocks from offset 0
    ///
    /// # Invariant
    /// - Buffers are contiguous from offset 0
    buffers: Vec<Bytes>,

    /// Total length cached (== sum of buffers lengths)
    len: u64,
}

impl SequentialCache {
    /// Create a new, empty SequentialCache
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
        let out_len = (range.end - range.start) as usize;
        // guaranteed valid
        let mut remaining = range;
        let mut out_buffers: Vec<Bytes> = vec![];

        for b in &self.buffers {
            let b_len = b.len() as u64;

            // this block falls entirely before the desired range start
            if remaining.start >= b_len {
                remaining.start -= b_len;
                remaining.end -= b_len;
                continue;
            }

            // we slice bytes out of *this* block
            let start = remaining.start as usize;
            let end = (remaining.end - remaining.start).min(b_len - remaining.start) as usize;

            let chunk = b.slice(start..end);
            out_buffers.push(chunk);

            // consumed some portion; update and potentially break
            remaining.start = 0;
            if remaining.end <= b_len {
                break;
            }
            remaining.end -= b_len;
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
pub struct ReadAheadMetadataCache<F: MetadataFetch> {
    inner: F,
    cache: Arc<Mutex<SequentialCache>>,

    initial_size: u64,
    increment_multiple: u64,
}

impl<F: MetadataFetch> ReadAheadMetadataCache<F> {
    /// Create a new EagerMetadataCache wrapping the given MetadataFetch
    pub fn new(inner: F) -> AsyncTiffResult<Self> {
        Self::with_increment_configuration(inner, 32 * 1024, 2)
    }

    /// Create a new EagerMetadataCache with custom initial size and increment multiple
    pub fn with_increment_configuration(
        inner: F,
        initial_size: u64,
        increment_multiple: u64,
    ) -> AsyncTiffResult<Self> {
        Ok(Self {
            inner,
            cache: Arc::new(Mutex::new(SequentialCache::new())),
            initial_size,
            increment_multiple,
        })
    }

    fn next_fetch_size(&self, current_len: u64) -> u64 {
        if current_len == 0 {
            self.initial_size
        } else {
            current_len * self.increment_multiple
        }
    }
}

impl<F: MetadataFetch + Send + Sync> MetadataFetch for ReadAheadMetadataCache<F> {
    fn fetch(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        let cache = self.cache.clone();

        Box::pin(async move {
            let mut cache = cache.lock().await;

            // First check if we already have the range cached
            if cache.contains(range.start..range.end) {
                return Ok(cache.slice(range));
            }

            // Compute the correct fetch range
            let start_len = cache.len;
            let needed = range.end.saturating_sub(start_len);
            let fetch_size = &self.next_fetch_size(start_len).max(needed);
            let fetch_range = start_len..start_len + fetch_size;

            // Perform the fetch while holding mutex
            // (this is OK because the mutex is async)
            let bytes = self.inner.fetch(fetch_range).await?;

            // Now append safely
            cache.append_buffer(bytes);

            Ok(cache.slice(range))
        })
    }
}
