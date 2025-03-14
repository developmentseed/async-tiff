use async_tiff::tiff::Value;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::IntoPyObjectExt;

pub struct PyValue(Value);

impl<'py> IntoPyObject<'py> for PyValue {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        match self.0 {
            Value::Byte(val) => val.into_bound_py_any(py),
            Value::Short(val) => val.into_bound_py_any(py),
            Value::SignedByte(val) => val.into_bound_py_any(py),
            Value::SignedShort(val) => val.into_bound_py_any(py),
            Value::Signed(val) => val.into_bound_py_any(py),
            Value::SignedBig(val) => val.into_bound_py_any(py),
            Value::Unsigned(val) => val.into_bound_py_any(py),
            Value::UnsignedBig(val) => val.into_bound_py_any(py),
            Value::Float(val) => val.into_bound_py_any(py),
            Value::Double(val) => val.into_bound_py_any(py),
            Value::List(val) => val
                .into_iter()
                .map(|v| PyValue(v).into_bound_py_any(py))
                .collect::<PyResult<Vec<_>>>()?
                .into_bound_py_any(py),
            Value::Rational(num, denom) => (num, denom).into_bound_py_any(py),
            Value::RationalBig(num, denom) => (num, denom).into_bound_py_any(py),
            Value::SRational(num, denom) => (num, denom).into_bound_py_any(py),
            Value::SRationalBig(num, denom) => (num, denom).into_bound_py_any(py),
            Value::Ascii(val) => val.into_bound_py_any(py),
            Value::Ifd(_val) => Err(PyRuntimeError::new_err("Unsupported value type 'Ifd'")),
            Value::IfdBig(_val) => Err(PyRuntimeError::new_err("Unsupported value type 'IfdBig'")),
            v => Err(PyRuntimeError::new_err(format!(
                "Unknown value type: {:?}",
                v
            ))),
        }
    }
}

impl From<Value> for PyValue {
    fn from(value: Value) -> Self {
        Self(value)
    }
}
