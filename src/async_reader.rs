use bytes::Bytes;
use futures::future::{BoxFuture, FutureExt, TryFutureExt};
use object_store::ObjectStore;
use std::io::SeekFrom;
use std::ops::Range;
use std::sync::Arc;

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
