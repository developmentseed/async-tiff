use pyo3::exceptions::PyTypeError;
use pyo3::intern;
use pyo3::prelude::*;
use pyo3::pybacked::PyBackedStr;

struct AiohttpSession(PyObject);

impl<'py> FromPyObject<'py> for AiohttpSession {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        let py = ob.py();
        let cls = ob.getattr(intern!(py, "__class__"))?;
        let module = cls
            .getattr(intern!(py, "__module__"))?
            .extract::<PyBackedStr>()?;
        let class_name = cls
            .getattr(intern!(py, "__name__"))?
            .extract::<PyBackedStr>()?;
        if module.starts_with("aiohttp") && class_name == "ClientSession" {
            todo!()
        } else {
            let msg = format!(
                "Expected aiohttp.ClientSession, got {}.{}",
                module, class_name
            );
            Err(PyTypeError::new_err(msg))
        }
    }
}
