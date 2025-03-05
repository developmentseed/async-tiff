use std::collections::HashMap;
use std::sync::Arc;

use async_tiff::decoder::{Decoder, DecoderRegistry};
use async_tiff::error::{AsyncTiffError, AsyncTiffResult};
use async_tiff::tiff::tags::PhotometricInterpretation;
use bytes::Bytes;
use pyo3::exceptions::PyTypeError;
use pyo3::intern;
use pyo3::prelude::*;
use pyo3::sync::GILOnceCell;
use pyo3::types::{PyDict, PyTuple};
use pyo3_bytes::PyBytes;

use crate::enums::PyCompressionMethod;

static DEFAULT_DECODER_REGISTRY: GILOnceCell<Arc<DecoderRegistry>> = GILOnceCell::new();

pub fn get_default_decoder_registry(py: Python<'_>) -> Arc<DecoderRegistry> {
    let registry =
        DEFAULT_DECODER_REGISTRY.get_or_init(py, || Arc::new(DecoderRegistry::default()));
    registry.clone()
}

#[pyclass(name = "DecoderRegistry", frozen)]
#[derive(Debug, Default)]
pub(crate) struct PyDecoderRegistry(Arc<DecoderRegistry>);

#[pymethods]
impl PyDecoderRegistry {
    #[new]
    #[pyo3(signature = (decoders = None))]
    pub(crate) fn new(decoders: Option<HashMap<PyCompressionMethod, PyDecoder>>) -> Self {
        let mut decoder_registry = DecoderRegistry::default();
        if let Some(decoders) = decoders {
            for (compression, decoder) in decoders.into_iter() {
                decoder_registry
                    .as_mut()
                    .insert(compression.into(), Box::new(decoder));
            }
        }
        Self(Arc::new(decoder_registry))
    }
}
impl PyDecoderRegistry {
    pub(crate) fn inner(&self) -> &Arc<DecoderRegistry> {
        &self.0
    }
}

#[derive(Debug)]
pub(crate) struct PyDecoder(PyObject);

impl PyDecoder {
    fn call(&self, py: Python, buffer: Bytes) -> PyResult<PyBytes> {
        let kwargs = PyDict::new(py);
        kwargs.set_item(intern!(py, "buffer"), PyBytes::new(buffer))?;
        let result = self.0.call(py, PyTuple::empty(py), Some(&kwargs))?;
        result.extract(py)
    }
}

impl<'py> FromPyObject<'py> for PyDecoder {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        if !ob.hasattr(intern!(ob.py(), "__call__"))? {
            return Err(PyTypeError::new_err(
                "Expected callable object for custom decoder.",
            ));
        }
        Ok(Self(ob.clone().unbind()))
    }
}

impl Decoder for PyDecoder {
    fn decode_tile(
        &self,
        buffer: Bytes,
        _photometric_interpretation: PhotometricInterpretation,
        _jpeg_tables: Option<&[u8]>,
    ) -> AsyncTiffResult<Bytes> {
        let decoded_buffer = Python::with_gil(|py| self.call(py, buffer))
            .map_err(|err| AsyncTiffError::General(err.to_string()))?;
        Ok(decoded_buffer.into_inner())
    }
}
