use async_tiff::decoder::{Decoder, DecoderRegistry};
use async_tiff::error::AiocogeoError;
use async_tiff::tiff::tags::PhotometricInterpretation;
use bytes::Bytes;
use pyo3::exceptions::PyTypeError;
use pyo3::intern;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use pyo3_bytes::PyBytes;

use crate::enums::PyCompressionMethod;

#[pyclass(name = "DecoderRegistry")]
pub(crate) struct PyDecoderRegistry(DecoderRegistry);

#[pymethods]
impl PyDecoderRegistry {
    #[new]
    fn new() -> Self {
        Self(DecoderRegistry::default())
    }

    fn add(&mut self, compression: PyCompressionMethod, decoder: PyDecoder) {
        self.0
            .as_mut()
            .insert(compression.into(), Box::new(decoder));
    }
}

#[derive(Debug)]
struct PyDecoder(PyObject);

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
        buffer: bytes::Bytes,
        _photometric_interpretation: PhotometricInterpretation,
        _jpeg_tables: Option<&[u8]>,
    ) -> async_tiff::error::Result<bytes::Bytes> {
        let decoded_buffer = Python::with_gil(|py| self.call(py, buffer))
            .map_err(|err| AiocogeoError::General(err.to_string()))?;
        Ok(decoded_buffer.into_inner())
    }
}
