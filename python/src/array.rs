//! A Python-exposed array type that implements the buffer protocol.
//!
//! This module provides `PyArray`, a 3D array type that can be used with numpy
//! via Python's buffer protocol. The buffer protocol allows Python objects to
//! expose raw memory buffers, enabling zero-copy interoperability with numpy.
//!
//! ## Buffer Protocol Overview
//!
//! The buffer protocol is defined in [PEP 3118] and allows objects to expose their
//! internal data as a contiguous or strided memory region. Key concepts:
//!
//! [PEP 3118](https://peps.python.org/pep-3118/)
//!
//! - **view**: A `Py_buffer` struct that describes how to interpret the memory
//! - **format**: A string describing the element type (e.g., "<H" for little-endian uint16)
//! - **shape**: Array dimensions (e.g., [height, width, bands] for a 3D array)
//! - **strides**: Byte offsets between consecutive elements in each dimension
//!
//! ## Reference Counting
//!
//! When `__getbuffer__` is called, we increment the reference count on the PyArray
//! object (`Py_INCREF`) to ensure it stays alive while the buffer view exists.
//! Python's `PyBuffer_Release` automatically calls `Py_DECREF` after `__releasebuffer__`
//! returns, so we don't need to manually decrement the count.
//!
//! ## Memory Safety
//!
//! The shape and strides arrays are stored as `Box<[isize; 3]>` on the PyArray struct
//! itself. This means their lifetime is tied to the PyArray object, which is kept
//! alive by the `Py_INCREF` call. This avoids the need to leak/free allocations
//! in `__getbuffer__`/`__releasebuffer__`.

use std::ffi::CStr;
use std::os::raw::c_int;

use async_tiff::{Array, DataType, TypedArray};
use pyo3::exceptions::PyValueError;
use pyo3::ffi;
use pyo3::prelude::*;
use pyo3_bytes::PyBytes;

use crate::error::PyAsyncTiffResult;

/// Returns the numpy dtype type character for this data type.
///
/// Numpy uses single characters to identify type families:
/// - 'u' = unsigned integer
/// - 'i' = signed integer
/// - 'f' = floating point
///
/// Combined with endianness and size, this forms a complete dtype string
/// like "<u2" (little-endian uint16) or ">f4" (big-endian float32).
#[expect(unused)]
fn data_type_to_numpy_char(dtype: &DataType) -> char {
    match dtype {
        // Represented as uint8 in numpy
        DataType::Bool => 'u',
        DataType::UInt8 => 'u',
        DataType::UInt16 => 'u',
        DataType::UInt32 => 'u',
        DataType::UInt64 => 'u',
        DataType::Int8 => 'i',
        DataType::Int16 => 'i',
        DataType::Int32 => 'i',
        DataType::Int64 => 'i',
        DataType::Float32 => 'f',
        DataType::Float64 => 'f',
    }
}

/// Returns the buffer protocol format string type character (without endianness prefix).
///
/// The format string uses Python's struct module syntax:
///   - 'B'/'b' = unsigned/signed 8-bit
///   - 'H'/'h' = unsigned/signed 16-bit
///   - 'I'/'i' = unsigned/signed 32-bit
///   - 'Q'/'q' = unsigned/signed 64-bit
///   - 'f'/'d' = 32/64-bit float
///
/// Note: These are distinct from numpy dtype strings! Numpy uses "<u2"
/// while the buffer protocol uses "<H" for the same type (little-endian uint16).
///
/// See: https://docs.python.org/3/library/struct.html#format-characters
fn data_type_to_buffer_format(data_type: &DataType) -> &'static CStr {
    use DataType::*;
    match data_type {
        Bool | UInt8 => c"B",
        UInt16 => c"H",
        UInt32 => c"I",
        UInt64 => c"Q",
        Int8 => c"b",
        Int16 => c"h",
        Int32 => c"i",
        Int64 => c"q",
        Float32 => c"f",
        Float64 => c"d",
    }
}

/// Parses a buffer protocol format string into a DataType.
///
/// Accepts format strings with optional endianness prefixes:
/// - '<' = little-endian
/// - '>' = big-endian
/// - '@', '=', '|' = native endianness
///
/// Followed by a type character:
/// - 'B'/'b' = unsigned/signed 8-bit
/// - 'H'/'h' = unsigned/signed 16-bit
/// - 'I'/'i' = unsigned/signed 32-bit
/// - 'Q'/'q' = unsigned/signed 64-bit
/// - 'f'/'d' = 32/64-bit float
///
/// Examples: "<H", ">f", "B", "@i"
///
/// Note: The endianness prefix is accepted but not returned. Callers should
/// validate that the actual data endianness matches separately if needed.
fn parse_buffer_format_string(s: &str) -> PyResult<DataType> {
    let mut chars = s.chars();

    // Check for optional endianness prefix
    let first_char = chars
        .next()
        .ok_or_else(|| PyValueError::new_err("empty format string"))?;

    // Determine if first character is an endianness prefix
    let type_char = match first_char {
        '<' | '>' | '@' | '=' | '|' => {
            // Endianness prefix found, next character should be the type
            chars
                .next()
                .ok_or_else(|| PyValueError::new_err("missing type character after endianness"))?
        }
        // No endianness prefix, first character is the type
        c => c,
    };

    let dtype = match type_char {
        'B' => DataType::UInt8,
        'H' => DataType::UInt16,
        'I' => DataType::UInt32,
        'Q' => DataType::UInt64,
        'b' => DataType::Int8,
        'h' => DataType::Int16,
        'i' => DataType::Int32,
        'q' => DataType::Int64,
        'f' => DataType::Float32,
        'd' => DataType::Float64,
        c => {
            return Err(PyValueError::new_err(format!(
                "invalid type character: '{c}'"
            )))
        }
    };

    // Ensure no extra characters after the type
    if chars.next().is_some() {
        return Err(PyValueError::new_err(format!(
            "unexpected characters after format: '{s}'"
        )));
    }

    Ok(dtype)
}

/// A 3D array that implements Python's buffer protocol.
///
/// This allows zero-copy interoperability with numpy via `np.asarray(arr)`.
/// The array is immutable (frozen) and exposes a read-only buffer.
///
/// See `_array.pyi` for Python usage examples and API documentation.
#[pyclass(name = "Array", frozen)]
pub struct PyArray {
    /// The raw data backing the array.
    data: TypedArray,

    /// The shape of the array as `[dim0, dim1, dim2]`.
    ///
    /// Stored as `isize` because the buffer protocol requires `Py_ssize_t` (= `isize`).
    ///
    /// The interpretation depends on the PlanarConfiguration:
    /// - PlanarConfiguration=1 (chunky): (height, width, bands)
    /// - PlanarConfiguration=2 (planar): (bands, height, width)
    shape: [isize; 3],

    /// Row-major (C-contiguous) strides in bytes.
    ///
    /// For a 3D array with shape [d0, d1, d2] and element size `itemsize`:
    /// - strides[0] = d1 * d2 * itemsize (bytes to skip for next row)
    /// - strides[1] = d2 * itemsize (bytes to skip for next column)
    /// - strides[2] = itemsize (bytes to skip for next element)
    strides: [isize; 3],

    /// The data type of array elements.
    data_type: DataType,
}

impl PyArray {
    pub(crate) fn try_new(array: Array) -> PyAsyncTiffResult<Self> {
        let (typed_data, shape, data_type) = array.into_inner();
        let data_type = data_type.ok_or(PyValueError::new_err(
            "Unknown data types are not currently supported.",
        ))?;

        let itemsize = data_type.size();
        let shape = [shape[0] as isize, shape[1] as isize, shape[2] as isize];
        // Row-major (C-contiguous) strides: [dim1 * dim2 * itemsize, dim2 * itemsize, itemsize]
        let strides = [
            (shape[1] as usize * shape[2] as usize * itemsize) as isize,
            (shape[2] as usize * itemsize) as isize,
            itemsize as isize,
        ];
        Ok(Self {
            data: typed_data,
            shape,
            strides,
            data_type,
        })
    }
}

#[pymethods]
impl PyArray {
    #[new]
    fn py_new(array: PyBytes, shape: [usize; 3], format: &str) -> PyResult<Self> {
        let data_type = parse_buffer_format_string(format)?;
        let itemsize = data_type.size();
        let typed_data = TypedArray::try_new(array.into_inner().to_vec(), Some(data_type)).unwrap();
        let shape = [shape[0] as isize, shape[1] as isize, shape[2] as isize];
        // Row-major (C-contiguous) strides: [dim1 * dim2 * itemsize, dim2 * itemsize, itemsize]
        let strides = [
            (shape[1] as usize * shape[2] as usize * itemsize) as isize,
            (shape[2] as usize * itemsize) as isize,
            itemsize as isize,
        ];
        Ok(Self {
            data: typed_data,
            shape,
            strides,
            data_type,
        })
    }

    #[getter]
    fn shape(&self) -> (isize, isize, isize) {
        (self.shape[0], self.shape[1], self.shape[2])
    }

    /// Implements the buffer protocol's `__getbuffer__` method (PEP 3118).
    ///
    /// This is called when Python code requests a buffer view of this object,
    /// for example via `memoryview(arr)` or `np.asarray(arr)`.
    ///
    /// ## Parameters
    ///
    /// - `slf`: Reference to the PyArray object
    /// - `view`: Pointer to a `Py_buffer` struct that we fill in
    /// - `flags`: Requested buffer capabilities (e.g., `PyBUF_FORMAT`, `PyBUF_STRIDES`)
    ///
    /// ## Safety
    ///
    /// This function is unsafe because we're working with raw pointers and FFI.
    /// We ensure safety by:
    ///
    /// 1. **Lifetime management**: We call `Py_INCREF` on the PyArray object to ensure
    ///    it stays alive as long as the buffer view exists. Python's `PyBuffer_Release`
    ///    will call `Py_DECREF` after `__releasebuffer__` returns.
    ///
    /// 2. **Stable pointers**: The `shape` and `strides` arrays are stored in `Box`es
    ///    on the PyArray struct, so their addresses remain stable and valid as long
    ///    as the PyArray exists.
    ///
    /// 3. **Read-only buffer**: We set `readonly = 1` so consumers cannot modify
    ///    our data through the buffer view.
    ///
    /// ## Buffer view fields
    ///
    /// - `buf`: Pointer to the raw data
    /// - `len`: Total size in bytes
    /// - `itemsize`: Size of one element in bytes
    /// - `readonly`: 1 = read-only, 0 = writable
    /// - `ndim`: Number of dimensions (3 for our 3D array)
    /// - `format`: Type format string (e.g., "<H" for little-endian uint16)
    /// - `shape`: Pointer to array of dimension sizes
    /// - `strides`: Pointer to array of byte strides per dimension
    /// - `suboffsets`: NULL (we don't use indirect addressing)
    /// - `internal`: NULL (reserved for exporter use)
    /// - `obj`: Reference to the exporting object (for reference counting)
    unsafe fn __getbuffer__(
        slf: PyRef<Self>,
        view: *mut ffi::Py_buffer,
        flags: c_int,
    ) -> PyResult<()> {
        let itemsize = slf.data_type.size();

        // Fill in the Py_buffer struct fields
        // SAFETY: view is a valid pointer provided by Python's buffer protocol machinery
        (*view).buf = data_as_ptr(&slf.data);
        (*view).len = slf.data.len() as isize;
        (*view).itemsize = itemsize as isize;
        (*view).readonly = 1; // Read-only buffer
        (*view).ndim = 3;

        // Only provide format string if requested (PyBUF_FORMAT flag)
        (*view).format = if flags & ffi::PyBUF_FORMAT != 0 {
            // SAFETY: data_type_to_buffer_format() returns a pointer to a static CStr
            data_type_to_buffer_format(&slf.data_type).as_ptr() as *mut std::ffi::c_char
        } else {
            std::ptr::null_mut()
        };

        // SAFETY: shape and strides are Box<[isize; 3]> owned by self.
        // The Py_INCREF below keeps self alive for the lifetime of this view.
        (*view).shape = slf.shape.as_ptr() as *mut isize;
        (*view).strides = slf.strides.as_ptr() as *mut isize;

        // We don't use indirect addressing (PIL-style)
        (*view).suboffsets = std::ptr::null_mut();
        // Reserved for internal use by the exporter
        (*view).internal = std::ptr::null_mut();

        // CRITICAL: Increment reference count to keep PyArray alive while buffer exists.
        // Python will call Py_DECREF after __releasebuffer__ returns.
        (*view).obj = slf.as_ptr();
        ffi::Py_INCREF((*view).obj);

        Ok(())
    }

    /// Called when a buffer view is released.
    ///
    /// For our implementation, this is a no-op because:
    /// - `shape` and `strides` are owned by the PyArray struct (not allocated per-view)
    /// - Python handles the `Py_DECREF` on `view.obj` automatically
    ///
    /// We still need to implement this method because PyO3 requires it when
    /// `__getbuffer__` is implemented.
    unsafe fn __releasebuffer__(&self, _view: *mut ffi::Py_buffer) {
        // Nothing to clean up - all memory is owned by the PyArray struct
    }
}

fn data_as_ptr(data: &TypedArray) -> *mut std::ffi::c_void {
    match data {
        // Bool is 1 byte per element with 0/1 values, same memory layout as u8
        TypedArray::Bool(data) => data.as_ptr() as _,
        TypedArray::UInt8(data) => data.as_ptr() as _,
        TypedArray::UInt16(data) => data.as_ptr() as _,
        TypedArray::UInt32(data) => data.as_ptr() as _,
        TypedArray::UInt64(data) => data.as_ptr() as _,
        TypedArray::Int8(data) => data.as_ptr() as _,
        TypedArray::Int16(data) => data.as_ptr() as _,
        TypedArray::Int32(data) => data.as_ptr() as _,
        TypedArray::Int64(data) => data.as_ptr() as _,
        TypedArray::Float32(data) => data.as_ptr() as _,
        TypedArray::Float64(data) => data.as_ptr() as _,
    }
}
