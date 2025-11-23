//! Caching strategies for metadata fetching.

use std::ops::Range;
use std::sync::Arc;

use bytes::{Bytes, BytesMut};
use futures::future::BoxFuture;
use tokio::sync::Mutex;

use crate::error::AsyncTiffResult;
use crate::metadata::MetadataFetch;

/// Logic for managing a cache of sequential buffers
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
            let size = (remaining.end - remaining.start).min(b_len - remaining.start) as usize;
            let end = start + size;

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
pub struct ReadaheadMetadataCache<F: MetadataFetch> {
    inner: F,
    cache: Arc<Mutex<SequentialBlockCache>>,
    initial: u64,
    multiplier: f64,
}

impl<F: MetadataFetch> ReadaheadMetadataCache<F> {
    /// Create a new ReadaheadMetadataCache wrapping the given MetadataFetch
    pub fn new(inner: F) -> AsyncTiffResult<Self> {
        Ok(Self {
            inner,
            cache: Arc::new(Mutex::new(SequentialBlockCache::new())),
            initial: 32 * 1024,
            multiplier: 2.0,
        })
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

impl<F: MetadataFetch + Send + Sync> MetadataFetch for ReadaheadMetadataCache<F> {
    fn fetch(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        Box::pin(async move {
            let mut g = self.cache.lock().await;

            // First check if we already have the range cached
            if g.contains(range.start..range.end) {
                return Ok(g.slice(range));
            }

            // Compute the correct fetch range
            let start_len = g.len;
            let needed = range.end.saturating_sub(start_len);
            let fetch_size = self.next_fetch_size(start_len).max(needed);
            let fetch_range = start_len..start_len + fetch_size;

            // Perform the fetch while holding mutex
            // (this is OK because the mutex is async)
            let bytes = self.inner.fetch(fetch_range).await?;

            // Now append safely
            g.append_buffer(bytes);

            Ok(g.slice(range))
        })
    }
}
