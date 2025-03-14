#![deny(clippy::undocumented_unsafe_blocks)]

mod decoder;
mod enums;
mod geo;
mod ifd;
mod thread_pool;
mod tiff;
mod tile;

use pyo3::prelude::*;

use crate::decoder::PyDecoderRegistry;
use crate::geo::PyGeoKeyDirectory;
use crate::ifd::PyImageFileDirectory;
use crate::thread_pool::PyThreadPool;
use crate::tiff::PyTIFF;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[pyfunction]
fn ___version() -> &'static str {
    VERSION
}

/// Raise RuntimeWarning for debug builds
#[pyfunction]
fn check_debug_build(_py: Python) -> PyResult<()> {
    #[cfg(debug_assertions)]
    {
        use pyo3::exceptions::PyRuntimeWarning;
        use pyo3::intern;
        use pyo3::types::PyTuple;

        let warnings_mod = _py.import(intern!(_py, "warnings"))?;
        let warning = PyRuntimeWarning::new_err(
            "async-tiff has not been compiled in release mode. Performance will be degraded.",
        );
        let args = PyTuple::new(_py, vec![warning])?;
        warnings_mod.call_method1(intern!(_py, "warn"), args)?;
    }

    Ok(())
}

#[pymodule]
fn _async_tiff(py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    check_debug_build(py)?;

    m.add_wrapped(wrap_pyfunction!(___version))?;
    m.add_class::<PyDecoderRegistry>()?;
    m.add_class::<PyGeoKeyDirectory>()?;
    m.add_class::<PyImageFileDirectory>()?;
    m.add_class::<PyThreadPool>()?;
    m.add_class::<PyTIFF>()?;

    pyo3_object_store::register_store_module(py, m, "async_tiff", "store")?;
    pyo3_object_store::register_exceptions_module(py, m, "async_tiff", "exceptions")?;

    Ok(())
}
