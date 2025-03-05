//! Error handling.

use std::fmt::Debug;
use thiserror::Error;

/// Enum with all errors in this crate.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum AsyncTiffError {
    /// End of file error.
    #[error("End of File: expected to read {0} bytes, got {1}")]
    EndOfFile(usize, usize),

    /// General error.
    #[error("General error: {0}")]
    General(String),

    /// IO Error.
    #[error(transparent)]
    IOError(#[from] std::io::Error),

    /// Error while decoding JPEG data.
    #[error(transparent)]
    JPEGDecodingError(#[from] jpeg::Error),

    /// Error while fetching data using object store.
    #[error(transparent)]
    ObjectStore(#[from] object_store::Error),

    /// An error during TIFF tag parsing.
    #[error(transparent)]
    InternalTIFFError(#[from] crate::tiff::TiffError),

    /// Reqwest error
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
}

/// Crate-specific result type.
pub type AsyncTiffResult<T> = std::result::Result<T, AsyncTiffError>;
