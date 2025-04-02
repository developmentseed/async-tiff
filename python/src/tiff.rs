use std::sync::Arc;

use async_tiff::metadata::{PrefetchBuffer, TiffMetadataReader};
use async_tiff::reader::AsyncFileReader;
use async_tiff::TIFF;
use pyo3::exceptions::PyIndexError;
use pyo3::prelude::*;
use pyo3::types::PyType;
use pyo3_async_runtimes::tokio::future_into_py;

use crate::reader::StoreInput;
use crate::tile::PyTile;
use crate::PyImageFileDirectory;

#[pyclass(name = "TIFF", frozen)]
pub(crate) struct PyTIFF {
    tiff: TIFF,
    reader: Box<dyn AsyncFileReader>,
}

#[pymethods]
impl PyTIFF {
    #[classmethod]
    #[pyo3(signature = (path, *, store, prefetch=32768))]
    fn open<'py>(
        _cls: &'py Bound<PyType>,
        py: Python<'py>,
        path: String,
        store: StoreInput,
        prefetch: u64,
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut reader = store.into_async_file_reader(path);

        let cog_reader = future_into_py(py, async move {
            let mut metadata_fetch = PrefetchBuffer::new(&mut reader, prefetch).await.unwrap();
            let mut metadata_reader = TiffMetadataReader::try_open(&mut metadata_fetch)
                .await
                .unwrap();
            let ifds = metadata_reader
                .read_all_ifds(&mut metadata_fetch)
                .await
                .unwrap();
            let tiff = TIFF::new(ifds);
            Ok(PyTIFF { tiff, reader })
        })?;
        Ok(cog_reader)
    }

    #[getter]
    fn ifds(&self) -> Vec<PyImageFileDirectory> {
        let ifds = self.tiff.ifds();
        ifds.as_ref().iter().map(|ifd| ifd.clone().into()).collect()
    }

    fn fetch_tile<'py>(
        &'py self,
        py: Python<'py>,
        x: usize,
        y: usize,
        z: usize,
    ) -> PyResult<Bound<'py, PyAny>> {
        let reader = &self.reader;
        let ifd = self
            .tiff
            .ifds()
            .as_ref()
            .get(z)
            .ok_or_else(|| PyIndexError::new_err(format!("No IFD found for z={}", z)))?
            // TODO: avoid this clone; add Arc to underlying rust code?
            .clone();
        future_into_py(py, async move {
            let tile = ifd.fetch_tile(x, y, reader.as_mut()).await.unwrap();
            Ok(PyTile::new(tile))
        })
    }

    fn fetch_tiles<'py>(
        &'py self,
        py: Python<'py>,
        x: Vec<usize>,
        y: Vec<usize>,
        z: usize,
    ) -> PyResult<Bound<'py, PyAny>> {
        let reader = self.reader.clone();
        let ifd = self
            .tiff
            .ifds()
            .as_ref()
            .get(z)
            .ok_or_else(|| PyIndexError::new_err(format!("No IFD found for z={}", z)))?
            // TODO: avoid this clone; add Arc to underlying rust code?
            .clone();
        future_into_py(py, async move {
            let tiles = ifd.fetch_tiles(&x, &y, reader.as_ref()).await.unwrap();
            let py_tiles = tiles.into_iter().map(PyTile::new).collect::<Vec<_>>();
            Ok(py_tiles)
        })
    }
}
