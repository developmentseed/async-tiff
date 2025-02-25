//! Support for GeoTIFF files.

mod affine;
mod geo_key_directory;
mod partial_reads;

pub use affine::AffineTransform;
pub use geo_key_directory::GeoKeyDirectory;
pub(crate) use geo_key_directory::GeoKeyTag;
