use async_tiff::Tile;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;
use pyo3_bytes::PyBytes;
use tokio_rayon::AsyncThreadPool;

use crate::array::PyArray;
use crate::decoder::get_default_decoder_registry;
use crate::enums::PyCompression;
use crate::error::PyAsyncTiffResult;
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
    fn compression_method(&self) -> PyResult<PyCompression> {
        self.0
            .as_ref()
            .ok_or(PyValueError::new_err("Tile has been consumed"))
            .map(|t| t.compression_method().into())
    }

    fn decode_sync<'py>(
        &mut self,
        py: Python<'py>,
        decoder_registry: Option<&PyDecoderRegistry>,
    ) -> PyAsyncTiffResult<PyArray> {
        let decoder_registry = decoder_registry
            .map(|r| r.inner().clone())
            .unwrap_or_else(|| get_default_decoder_registry(py));
        let tile = self
            .0
            .take()
            .ok_or(PyValueError::new_err("Tile has been consumed"))?;
        let array = tile.decode(&decoder_registry)?;
        PyArray::try_new(array)
    }

    #[pyo3(signature = (*, decoder_registry=None, pool=None))]
    fn decode<'py>(
        &mut self,
        py: Python<'py>,
        decoder_registry: Option<&PyDecoderRegistry>,
        pool: Option<&PyThreadPool>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let decoder_registry = decoder_registry
            .map(|r| r.inner().clone())
            .unwrap_or_else(|| get_default_decoder_registry(py));
        let pool = pool
            .map(|p| Ok(p.inner().clone()))
            .unwrap_or_else(|| get_default_pool(py))?;
        let tile = self
            .0
            .take()
            .ok_or(PyValueError::new_err("Tile has been consumed"))?;

        future_into_py(py, async move {
            let array = pool
                .spawn_fifo_async(move || tile.decode(&decoder_registry))
                .await
                .map_err(|e| PyValueError::new_err(e.to_string()))?;
            PyArray::try_new(array).map_err(|err| err.into())
        })
    }
}
