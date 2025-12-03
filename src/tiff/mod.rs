//! Vendored content from tiff crate

mod error;
mod ifd;

pub(crate) use error::{TiffError, TiffFormatError, TiffResult, TiffUnsupportedError};
pub use ifd::Value;
