//! TIFF metadata API

mod fetch;
mod reader;

pub use fetch::{MetadataFetch, PrefetchMetadataFetch};
pub use reader::{ImageFileDirectoryReader, TiffMetadataReader};
