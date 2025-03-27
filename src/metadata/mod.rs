//! API for reading metadata out of a TIFF file.
//!
//! ### Reading all TIFF metadata
//!
//! We can use [`TiffMetadataReader::read_all_ifds`] to read all IFDs up front:
//!
//! ```
//! use std::env::current_dir;
//! use std::sync::Arc;
//!
//! use object_store::local::LocalFileSystem;
//!
//! use async_tiff::metadata::{PrefetchMetadataFetch, TiffMetadataReader};
//! use async_tiff::reader::ObjectReader;
//!
//! // Create new Arc<dyn ObjectStore>
//! let store = Arc::new(LocalFileSystem::new_with_prefix(current_dir().unwrap()).unwrap());
//!
//! // Create new ObjectReader to map the ObjectStore to the AsyncFileReader trait
//! let reader = ObjectReader::new(
//!     store,
//!     "tests/image_tiff/images/tiled-jpeg-rgb-u8.tif".into(),
//! );
//!
//! // Use PrefetchMetadataFetch to ensure that a given number of bytes at the start of the
//! // file are prefetched.
//! //
//! // This or a similar caching layer should **always** be used and ensures that the
//! // underlying read calls that the TiffMetadataReader makes don't translate to actual
//! // network fetches.
//! let prefetch_reader = PrefetchMetadataFetch::new(reader.clone(), 32 * 1024)
//!     .await
//!     .unwrap();
//!
//! // Create a TiffMetadataReader wrapping some MetadataFetch
//! let mut metadata_reader = TiffMetadataReader::try_open(&prefetch_reader)
//!     .await
//!     .unwrap();
//!
//! // Read all IFDs out of the source.
//! let ifds = metadata_reader
//!     .read_all_ifds(&prefetch_reader)
//!     .await
//!     .unwrap();
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
//! middleware when reading metadata. [`PrefetchMetadataFetch`] is an example of this, which
//! fetches the first `N` bytes out of a file.
//!

mod fetch;
mod reader;

pub use fetch::{MetadataFetch, PrefetchMetadataFetch};
pub use reader::{ImageFileDirectoryReader, TiffMetadataReader};
