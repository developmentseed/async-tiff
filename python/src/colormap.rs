//! A 2D u16 array exposing the TIFF colormap via the buffer protocol.

use std::os::raw::c_int;
use std::sync::Arc;

use pyo3::ffi;
use pyo3::prelude::*;

/// A 2D array of u16 values representing a TIFF colormap.
///
/// Exposed as shape `(N, 3)` where N is the number of color entries.
/// Access as `colormap[pixel_value]` to get `[R, G, B]` for that index.
///
/// Implements Python's buffer protocol for zero-copy access via `np.asarray()`.
#[pyclass(name = "Colormap", frozen)]
pub struct PyColormap {
    data: Arc<[u16]>,
    /// Shape array for buffer protocol: [num_entries, 3]
    shape: [isize; 2],
    /// Strides in bytes: [2, num_entries * 2] for (N, 3) layout
    strides: [isize; 2],
}

impl PyColormap {
    pub fn new(data: Arc<[u16]>) -> Self {
        let num_entries = (data.len() / 3) as isize;
        let shape = [num_entries, 3];
        // Strides for (N, 3) view of [R0..RN, G0..GN, B0..BN] data:
        // - To go to next pixel: +2 bytes (1 u16)
        // - To go to next channel: +num_entries * 2 bytes
        let strides = [2, num_entries * 2];
        Self {
            data,
            shape,
            strides,
        }
    }
}

#[pymethods]
impl PyColormap {
    fn __len__(&self) -> usize {
        self.shape[0] as usize
    }

    // SAFETY: We expose a read-only buffer view of data owned by self.data (Arc<[u16]>).
    // The Py_INCREF ensures self stays alive for the buffer's lifetime.
    // shape and strides are stored on self, which is kept alive by the ref count.
    #[allow(unsafe_code)]
    unsafe fn __getbuffer__(
        slf: PyRef<Self>,
        view: *mut ffi::Py_buffer,
        flags: c_int,
    ) -> PyResult<()> {
        (*view).buf = slf.data.as_ptr() as *mut _;
        (*view).len = (slf.data.len() * 2) as isize;
        (*view).itemsize = 2;
        (*view).readonly = 1;
        (*view).ndim = 2;
        (*view).format = if flags & ffi::PyBUF_FORMAT != 0 {
            c"H".as_ptr() as *mut _
        } else {
            std::ptr::null_mut()
        };
        (*view).shape = slf.shape.as_ptr() as *mut _;
        (*view).strides = slf.strides.as_ptr() as *mut _;
        (*view).suboffsets = std::ptr::null_mut();
        (*view).internal = std::ptr::null_mut();
        (*view).obj = slf.as_ptr();
        ffi::Py_INCREF((*view).obj);

        Ok(())
    }

    #[allow(unsafe_code)]
    unsafe fn __releasebuffer__(&self, _view: *mut ffi::Py_buffer) {}
}
