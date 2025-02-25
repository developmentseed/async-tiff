use async_tiff::{AsyncFileReader, COGReader, ObjectReader, PrefetchReader};
use pyo3::prelude::*;
use pyo3::types::PyType;
use pyo3_async_runtimes::tokio::future_into_py;
use pyo3_object_store::PyObjectStore;

use crate::PyImageFileDirectory;

#[pyclass(name = "TIFF", frozen)]
pub(crate) struct PyTIFF(COGReader);

#[pymethods]
impl PyTIFF {
    #[classmethod]
    #[pyo3(signature = (path, *, store, prefetch=16384))]
    fn open<'py>(
        _cls: &'py Bound<PyType>,
        py: Python<'py>,
        path: String,
        store: PyObjectStore,
        prefetch: Option<u64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let reader = ObjectReader::new(store.into_inner(), path.into());

        let cog_reader = future_into_py(py, async move {
            let reader: Box<dyn AsyncFileReader> = if let Some(prefetch) = prefetch {
                Box::new(
                    PrefetchReader::new(Box::new(reader), prefetch)
                        .await
                        .unwrap(),
                )
            } else {
                Box::new(reader)
            };
            Ok(PyTIFF(COGReader::try_open(reader).await.unwrap()))
        })?;
        Ok(cog_reader)
    }

    fn ifds(&self) -> Vec<PyImageFileDirectory> {
        let ifds = self.0.ifds();
        ifds.as_ref().iter().map(|ifd| ifd.clone().into()).collect()
    }
}
