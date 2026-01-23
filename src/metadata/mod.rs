//! API for reading metadata out of a TIFF file.
//!
//! ### Reading all TIFF metadata
//!
//! We can use [`TiffMetadataReader::read_all_ifds`] to read all IFDs up front:
//!
//! ```
//! # tokio_test::block_on(async {
//! use std::env::current_dir;
//! use std::sync::Arc;
//!
//! use object_store::local::LocalFileSystem;
//!
//! use async_tiff::metadata::TiffMetadataReader;
//! use async_tiff::metadata::cache::ReadaheadMetadataCache;
//! use async_tiff::reader::ObjectReader;
//!
//! // Create new Arc<dyn ObjectStore>
//! let store = Arc::new(LocalFileSystem::new_with_prefix(current_dir().unwrap()).unwrap());
//!
//! // Create new ObjectReader to map the ObjectStore to the AsyncFileReader trait
//! let reader = ObjectReader::new(
//!     store,
//!     "fixtures/image-tiff/tiled-jpeg-rgb-u8.tif".into(),
//! );
//!
//! // Use ReadaheadMetadataCache to ensure that a given number of bytes at the start of the
//! // file are prefetched, and to ensure that any additional fetches are made in larger chunks.
//! //
//! // The `ReadaheadMetadataCache` or a similar caching layer should **always** be used to ensure
//! // that the underlying small read calls that the TiffMetadataReader makes don't translate to
//! // individual tiny network fetches.
//! let cached_reader = ReadaheadMetadataCache::new(reader.clone());
//!
//! // Create a TiffMetadataReader wrapping some MetadataFetch
//! let mut metadata_reader = TiffMetadataReader::try_open(&cached_reader)
//!     .await
//!     .unwrap();
//!
//! // Read all IFDs out of the source.
//! let ifds = metadata_reader
//!     .read_all_ifds(&cached_reader)
//!     .await
//!     .unwrap();
//! # })
//! ```
//!
//!
//! ### Caching/prefetching/buffering
//!
//! The underlying [`ImageFileDirectoryReader`] used to read tags out of the TIFF file reads each
//! tag individually. This means that it will make many small byte range requests to the
//! [`MetadataFetch`] implementation.
//!
//! Thus, it is **imperative to always supply some sort of caching, prefetching, or buffering**
//! middleware when reading metadata. [`ReadaheadMetadataCache`] is an example of this, which
//! fetches the first `N` bytes out of a file, and then multiplies the size of any subsequent
//! fetches by a given `multiplier`.

pub mod cache;
mod fetch;
mod reader;

pub use fetch::MetadataFetch;
pub use reader::{ImageFileDirectoryReader, TiffMetadataReader};
