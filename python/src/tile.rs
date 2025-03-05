use async_tiff::Tile;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;
use pyo3_bytes::PyBytes;
use tokio_rayon::AsyncThreadPool;

use crate::decoder::get_default_decoder_registry;
use crate::enums::PyCompressionMethod;
use crate::thread_pool::{get_default_pool, PyThreadPool};
use crate::PyDecoderRegistry;

#[pyclass(name = "Tile")]
pub(crate) struct PyTile(Option<Tile>);

impl PyTile {
    pub(crate) fn new(tile: Tile) -> Self {
        Self(Some(tile))
    }
}

#[pymethods]
impl PyTile {
    #[getter]
    fn x(&self) -> PyResult<usize> {
        self.0
            .as_ref()
            .ok_or(PyValueError::new_err("Tile has been consumed"))
            .map(|t| t.x())
    }

    #[getter]
    fn y(&self) -> PyResult<usize> {
        self.0
            .as_ref()
            .ok_or(PyValueError::new_err("Tile has been consumed"))
            .map(|t| t.y())
    }

    #[getter]
    fn compressed_bytes(&self) -> PyResult<PyBytes> {
        let tile = self
            .0
            .as_ref()
            .ok_or(PyValueError::new_err("Tile has been consumed"))?;
        Ok(tile.compressed_bytes().clone().into())
    }

    #[getter]
    fn compression_method(&self) -> PyResult<PyCompressionMethod> {
        self.0
            .as_ref()
            .ok_or(PyValueError::new_err("Tile has been consumed"))
            .map(|t| t.compression_method().into())
    }

    #[pyo3(signature = (*, decoder_registry=None, pool=None))]
    fn decode_async(
        &mut self,
        py: Python,
        decoder_registry: Option<&PyDecoderRegistry>,
        pool: Option<&PyThreadPool>,
    ) -> PyResult<PyObject> {
        let decoder_registry = decoder_registry
            .map(|r| r.inner().clone())
            .unwrap_or_else(|| get_default_decoder_registry(py));
        let pool = pool
            .map(|p| Ok(p.inner().clone()))
            .unwrap_or_else(|| get_default_pool(py))?;
        let tile = self.0.take().unwrap();

        let result = future_into_py(py, async move {
            let decoded_bytes = pool
                .spawn_async(move || tile.decode(&decoder_registry))
                .await
                .unwrap();
            Ok(PyBytes::new(decoded_bytes))
        })?;
        Ok(result.unbind())
    }
}
