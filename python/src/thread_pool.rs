use std::sync::Arc;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use pyo3::sync::PyOnceLock;
use rayon::{ThreadPool, ThreadPoolBuilder};

static DEFAULT_POOL: PyOnceLock<Arc<ThreadPool>> = PyOnceLock::new();

pub fn get_default_pool(py: Python<'_>) -> PyResult<Arc<ThreadPool>> {
    let runtime = DEFAULT_POOL.get_or_try_init(py, || {
        let pool = ThreadPoolBuilder::new().build().map_err(|err| {
            PyValueError::new_err(format!("Could not create rayon threadpool. {err}"))
        })?;
        Ok::<_, PyErr>(Arc::new(pool))
    })?;
    Ok(runtime.clone())
}

#[pyclass(name = "ThreadPool", frozen, module = "async_tiff")]
pub(crate) struct PyThreadPool(Arc<ThreadPool>);

#[pymethods]
impl PyThreadPool {
    #[new]
    fn new(num_threads: usize) -> PyResult<Self> {
        let pool = ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .map_err(|err| {
                PyValueError::new_err(format!("Could not create rayon threadpool. {err}"))
            })?;
        Ok(Self(Arc::new(pool)))
    }
}

impl PyThreadPool {
    pub(crate) fn inner(&self) -> &Arc<ThreadPool> {
        &self.0
    }
}

impl AsRef<ThreadPool> for PyThreadPool {
    fn as_ref(&self) -> &ThreadPool {
        &self.0
    }
}
