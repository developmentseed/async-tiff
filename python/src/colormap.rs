//! A 1D u16 array exposing the TIFF colormap via the buffer protocol.

use std::os::raw::c_int;
use std::sync::Arc;

use pyo3::ffi;
use pyo3::prelude::*;

/// A 1D array of u16 values representing a TIFF colormap.
///
/// Implements Python's buffer protocol for zero-copy access via `np.asarray()`.
#[pyclass(name = "Colormap", frozen)]
pub struct PyColormap {
    data: Arc<[u16]>,
}

impl PyColormap {
    pub fn new(data: Arc<[u16]>) -> Self {
        Self { data }
    }
}

#[pymethods]
impl PyColormap {
    fn __len__(&self) -> usize {
        self.data.len()
    }

    unsafe fn __getbuffer__(
        slf: PyRef<Self>,
        view: *mut ffi::Py_buffer,
        flags: c_int,
    ) -> PyResult<()> {
        (*view).buf = slf.data.as_ptr() as *mut _;
        (*view).len = (slf.data.len() * 2) as isize;
        (*view).itemsize = 2;
        (*view).readonly = 1;
        (*view).ndim = 1;
        (*view).format = if flags & ffi::PyBUF_FORMAT != 0 {
            c"H".as_ptr() as *mut _
        } else {
            std::ptr::null_mut()
        };
        // For shape, we need a stable pointer. Use internal to store the length.
        (*view).internal = slf.data.len() as *mut _;
        (*view).shape = &(*view).internal as *const *mut _ as *mut isize;
        (*view).strides = &(*view).itemsize as *const isize as *mut _;
        (*view).suboffsets = std::ptr::null_mut();
        (*view).obj = slf.as_ptr();
        ffi::Py_INCREF((*view).obj);

        Ok(())
    }

    unsafe fn __releasebuffer__(&self, _view: *mut ffi::Py_buffer) {}
}
