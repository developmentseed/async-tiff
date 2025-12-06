use async_tiff::TagValue;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::IntoPyObjectExt;

pub struct PyValue(TagValue);

impl<'py> IntoPyObject<'py> for PyValue {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        match self.0 {
            TagValue::Byte(val) => val.into_bound_py_any(py),
            TagValue::Short(val) => val.into_bound_py_any(py),
            TagValue::SignedByte(val) => val.into_bound_py_any(py),
            TagValue::SignedShort(val) => val.into_bound_py_any(py),
            TagValue::Signed(val) => val.into_bound_py_any(py),
            TagValue::SignedBig(val) => val.into_bound_py_any(py),
            TagValue::Unsigned(val) => val.into_bound_py_any(py),
            TagValue::UnsignedBig(val) => val.into_bound_py_any(py),
            TagValue::Float(val) => val.into_bound_py_any(py),
            TagValue::Double(val) => val.into_bound_py_any(py),
            TagValue::List(val) => val
                .into_iter()
                .map(|v| PyValue(v).into_bound_py_any(py))
                .collect::<PyResult<Vec<_>>>()?
                .into_bound_py_any(py),
            TagValue::Rational(num, denom) => (num, denom).into_bound_py_any(py),
            TagValue::RationalBig(num, denom) => (num, denom).into_bound_py_any(py),
            TagValue::SRational(num, denom) => (num, denom).into_bound_py_any(py),
            TagValue::SRationalBig(num, denom) => (num, denom).into_bound_py_any(py),
            TagValue::Ascii(val) => val.into_bound_py_any(py),
            TagValue::Ifd(_val) => Err(PyRuntimeError::new_err("Unsupported value type 'Ifd'")),
            TagValue::IfdBig(_val) => {
                Err(PyRuntimeError::new_err("Unsupported value type 'IfdBig'"))
            }
            v => Err(PyRuntimeError::new_err(format!(
                "Unknown value type: {v:?}"
            ))),
        }
    }
}

impl From<TagValue> for PyValue {
    fn from(value: TagValue) -> Self {
        Self(value)
    }
}
