use std::ops::Range;
use std::sync::Arc;

use async_tiff::error::{AsyncTiffError, AsyncTiffResult};
use async_tiff::reader::{AsyncFileReader, ObjectReader};
use bytes::Bytes;
use futures::future::BoxFuture;
use futures::FutureExt;
use pyo3::exceptions::PyTypeError;
use pyo3::intern;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_async_runtimes::tokio::into_future;
use pyo3_bytes::PyBytes;
use pyo3_object_store::PyObjectStore;

#[derive(FromPyObject)]
pub(crate) enum StoreInput {
    ObjectStore(PyObjectStore),
    ObspecBackend(ObspecBackend),
}

impl StoreInput {
    pub(crate) fn into_async_file_reader(self, path: String) -> Arc<dyn AsyncFileReader> {
        match self {
            Self::ObjectStore(store) => {
                Arc::new(ObjectReader::new(store.into_inner(), path.into()))
            }
            Self::ObspecBackend(backend) => Arc::new(ObspecReader { backend, path }),
        }
    }
}

/// A Python backend for making requests that conforms to the GetRangeAsync and GetRangesAsync
/// protocols defined by obspec.
/// https://developmentseed.org/obspec/latest/api/get/#obspec.GetRangeAsync
/// https://developmentseed.org/obspec/latest/api/get/#obspec.GetRangesAsync
#[derive(Debug)]
pub(crate) struct ObspecBackend(PyObject);

impl ObspecBackend {
    async fn get_range(&self, path: &str, range: Range<u64>) -> PyResult<PyBytes> {
        let future = Python::with_gil(|py| {
            let kwargs = PyDict::new(py);
            kwargs.set_item(intern!(py, "path"), path)?;
            kwargs.set_item(intern!(py, "start"), range.start)?;
            kwargs.set_item(intern!(py, "end"), range.end)?;

            let coroutine = self
                .0
                .call_method(py, intern!(py, "get_range"), (), Some(&kwargs))?;
            into_future(coroutine.bind(py).clone())
        })?;
        let result = future.await?;
        Python::with_gil(|py| result.extract(py))
    }

    async fn get_ranges(&self, path: &str, ranges: &[Range<u64>]) -> PyResult<Vec<PyBytes>> {
        let starts = ranges.iter().map(|r| r.start).collect::<Vec<_>>();
        let ends = ranges.iter().map(|r| r.end).collect::<Vec<_>>();

        let future = Python::with_gil(|py| {
            let kwargs = PyDict::new(py);
            kwargs.set_item(intern!(py, "path"), path)?;
            kwargs.set_item(intern!(py, "starts"), starts)?;
            kwargs.set_item(intern!(py, "ends"), ends)?;

            let coroutine = self
                .0
                .call_method(py, intern!(py, "get_range"), (), Some(&kwargs))?;
            into_future(coroutine.bind(py).clone())
        })?;
        let result = future.await?;
        Python::with_gil(|py| result.extract(py))
    }

    async fn get_range_wrapper(&self, path: &str, range: Range<u64>) -> AsyncTiffResult<Bytes> {
        let result = self
            .get_range(path, range)
            .await
            .map_err(|err| AsyncTiffError::External(Box::new(err)))?;
        Ok(result.into_inner())
    }

    async fn get_ranges_wrapper(
        &self,
        path: &str,
        ranges: Vec<Range<u64>>,
    ) -> AsyncTiffResult<Vec<Bytes>> {
        let result = self
            .get_ranges(path, &ranges)
            .await
            .map_err(|err| AsyncTiffError::External(Box::new(err)))?;
        Ok(result.into_iter().map(|b| b.into_inner()).collect())
    }
}

impl<'py> FromPyObject<'py> for ObspecBackend {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        let py = ob.py();
        if ob.hasattr(intern!(py, "get_range_async"))?
            && ob.hasattr(intern!(py, "get_ranges_async"))?
        {
            Ok(Self(ob.clone().unbind()))
        } else {
            Err(PyTypeError::new_err("Expected obspec-compatible class with `get_range_async` and `get_ranges_async` method."))
        }
    }
}

#[derive(Debug)]
struct ObspecReader {
    backend: ObspecBackend,
    path: String,
}

impl AsyncFileReader for ObspecReader {
    fn get_bytes(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        self.backend.get_range_wrapper(&self.path, range).boxed()
    }

    fn get_byte_ranges(
        &self,
        ranges: Vec<Range<u64>>,
    ) -> BoxFuture<'_, AsyncTiffResult<Vec<Bytes>>> {
        self.backend.get_ranges_wrapper(&self.path, ranges).boxed()
    }
}
