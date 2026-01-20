use pyo3::ffi;
use pyo3::prelude::*;
use std::os::raw::c_int;

#[pyfunction]
pub fn example_array() -> PyArray {
    PyArray {
        data: vec![1, 2, 3, 4, 5, 6],
        shape: [2, 3], // 2 rows, 3 columns
    }
}

#[pyclass]
pub struct PyArray {
    data: Vec<u8>,
    shape: [usize; 2], // (height, width)
}

#[pymethods]
impl PyArray {
    #[new]
    fn new(height: usize, width: usize) -> Self {
        Self {
            data: vec![0u8; height * width],
            shape: [height, width],
        }
    }

    #[getter]
    fn shape(&self) -> (usize, usize) {
        (self.shape[0], self.shape[1])
    }

    #[getter]
    fn data(&self) -> PyResult<Vec<u8>> {
        Ok(self.data.clone())
    }

    // SAFETY: We carefully manage the Py_buffer fields and ensure that:
    // - shape_ptr points to data owned by the PyArray (self.shape)
    // - strides are stored in a Box that is leaked and freed in __releasebuffer__
    // - The PyArray object is kept alive via Py_INCREF on view.obj
    #[allow(unsafe_code)]
    unsafe fn __getbuffer__(
        slf: PyRef<Self>,
        view: *mut ffi::Py_buffer,
        flags: c_int,
    ) -> PyResult<()> {
        let bytes_per_element = std::mem::size_of::<u8>();

        // SAFETY: view is a valid pointer provided by Python
        (*view).buf = slf.data.as_ptr() as *mut std::ffi::c_void;
        (*view).len = slf.data.len() as isize;
        (*view).itemsize = bytes_per_element as isize;
        (*view).readonly = 1;
        (*view).ndim = 2;
        (*view).format = if flags & ffi::PyBUF_FORMAT != 0 {
            // "B" = unsigned char (uint8)
            // SAFETY: c"B" is a valid null-terminated string
            c"B".as_ptr() as *mut std::ffi::c_char
        } else {
            std::ptr::null_mut()
        };

        // SAFETY: shape is owned by self which is kept alive via Py_INCREF below
        (*view).shape = slf.shape.as_ptr() as *mut isize;

        // Row-major (C-contiguous) strides: [width * itemsize, itemsize]
        let strides: Box<[isize; 2]> = Box::new([
            (slf.shape[1] * bytes_per_element) as isize,
            bytes_per_element as isize,
        ]);
        // SAFETY: We leak this allocation and free it in __releasebuffer__
        (*view).strides = Box::leak(strides).as_ptr() as *mut isize;

        (*view).suboffsets = std::ptr::null_mut();
        (*view).internal = std::ptr::null_mut();

        // SAFETY: Keep the PyArray alive for the duration of the buffer view
        (*view).obj = slf.as_ptr() as *mut ffi::PyObject;
        ffi::Py_INCREF((*view).obj);

        Ok(())
    }

    // SAFETY: We free the strides allocation that was leaked in __getbuffer__
    #[allow(unsafe_code)]
    unsafe fn __releasebuffer__(&self, view: *mut ffi::Py_buffer) {
        // SAFETY: strides was allocated with Box::leak in __getbuffer__
        if !(*view).strides.is_null() {
            drop(Box::from_raw((*view).strides as *mut [isize; 2]));
        }
    }
}
