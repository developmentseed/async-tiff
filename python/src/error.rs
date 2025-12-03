use async_tiff::error::AsyncTiffError;
use pyo3::create_exception;
use pyo3::exceptions::PyFileNotFoundError;
use pyo3::prelude::*;

create_exception!(
    async_tiff,
    AsyncTiffException,
    pyo3::exceptions::PyException,
    "A general error from the underlying Rust async-tiff library."
);

#[allow(missing_docs)]
pub enum PyAsyncTiffError {
    AsyncTiffError(async_tiff::error::AsyncTiffError),
}

impl From<AsyncTiffError> for PyAsyncTiffError {
    fn from(value: AsyncTiffError) -> Self {
        Self::AsyncTiffError(value)
    }
}

impl From<PyAsyncTiffError> for PyErr {
    fn from(error: PyAsyncTiffError) -> Self {
        match error {
            PyAsyncTiffError::AsyncTiffError(err) => match err {
                AsyncTiffError::ObjectStore(err) => match err {
                    object_store::Error::NotFound { path: _, source: _ } => {
                        PyFileNotFoundError::new_err(err.to_string())
                    }
                    _ => AsyncTiffException::new_err(err.to_string()),
                },
                _ => AsyncTiffException::new_err(err.to_string()),
            },
        }
    }
}

pub type PyAsyncTiffResult<T> = std::result::Result<T, PyAsyncTiffError>;
