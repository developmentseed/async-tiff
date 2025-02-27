//! Vendored content from tiff crate

mod error;
mod ifd;
pub mod tags;

pub(crate) use error::{TiffError, TiffFormatError, TiffResult, TiffUnsupportedError};
pub(crate) use ifd::Value;
