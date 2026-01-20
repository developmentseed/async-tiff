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
use std::fmt::Display;
use std::os::raw::c_int;
use std::str::FromStr;

use bytes::Bytes;
use pyo3::exceptions::PyValueError;
use pyo3::ffi;
use pyo3::prelude::*;
use pyo3_bytes::PyBytes;

/// Supported numeric data types for array elements.
#[derive(Debug, Clone, Copy)]
enum DataType {
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Int8,
    Int16,
    Int32,
    Int64,
    Float32,
    Float64,
}

impl DataType {
    /// Returns the numpy dtype type character for this data type.
    ///
    /// Numpy uses single characters to identify type families:
    /// - 'u' = unsigned integer
    /// - 'i' = signed integer
    /// - 'f' = floating point
    ///
    /// Combined with endianness and size, this forms a complete dtype string
    /// like "<u2" (little-endian uint16) or ">f4" (big-endian float32).
    fn numpy_char(&self) -> char {
        match self {
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

    /// The size in bytes of this data type.
    fn size(&self) -> usize {
        match self {
            DataType::UInt8 => 1,
            DataType::UInt16 => 2,
            DataType::UInt32 => 4,
            DataType::UInt64 => 8,
            DataType::Int8 => 1,
            DataType::Int16 => 2,
            DataType::Int32 => 4,
            DataType::Int64 => 8,
            DataType::Float32 => 4,
            DataType::Float64 => 8,
        }
    }
}

/// Byte order for multi-byte data types.
enum Endianness {
    Little,
    Big,
}

impl Endianness {
    /// Returns the character used in numpy dtype strings for this endianness.
    /// '<' = little-endian, '>' = big-endian
    fn char(&self) -> char {
        match self {
            Endianness::Little => '<',
            Endianness::Big => '>',
        }
    }
}

/// Combined data type and endianness information.
///
/// This is used both for parsing format strings from Python and for
/// generating the format string to expose via the buffer protocol.
struct DataTypeAndEndianness {
    dtype: DataType,
    endianness: Endianness,
}

impl DataTypeAndEndianness {
    /// Returns the buffer protocol format string including endianness.
    ///
    /// The format string uses Python's struct module syntax:
    /// - First character: endianness ('<' = little, '>' = big)
    /// - Second character: type code from struct module
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
    fn buffer_format(&self) -> &'static CStr {
        use DataType::*;
        use Endianness::*;
        match (&self.endianness, &self.dtype) {
            // Little-endian
            (Little, UInt8) => c"<B",
            (Little, UInt16) => c"<H",
            (Little, UInt32) => c"<I",
            (Little, UInt64) => c"<Q",
            (Little, Int8) => c"<b",
            (Little, Int16) => c"<h",
            (Little, Int32) => c"<i",
            (Little, Int64) => c"<q",
            (Little, Float32) => c"<f",
            (Little, Float64) => c"<d",
            // Big-endian
            (Big, UInt8) => c">B",
            (Big, UInt16) => c">H",
            (Big, UInt32) => c">I",
            (Big, UInt64) => c">Q",
            (Big, Int8) => c">b",
            (Big, Int16) => c">h",
            (Big, Int32) => c">i",
            (Big, Int64) => c">q",
            (Big, Float32) => c">f",
            (Big, Float64) => c">d",
        }
    }
}

/// This parsing implementation is done for use in test cases
impl FromStr for DataTypeAndEndianness {
    type Err = PyErr;

    /// Parse a buffer protocol format string like "<H" or ">f".
    ///
    /// Supported endianness prefixes:
    /// - '<' = little-endian
    /// - '>' = big-endian
    /// - '@', '=', '|' = native endianness (resolved at compile time)
    ///
    /// Supported type characters (from Python's struct module):
    /// - 'B'/'b' = unsigned/signed 8-bit integer
    /// - 'H'/'h' = unsigned/signed 16-bit integer
    /// - 'I'/'i' = unsigned/signed 32-bit integer
    /// - 'Q'/'q' = unsigned/signed 64-bit integer
    /// - 'f' = 32-bit float
    /// - 'd' = 64-bit float
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();

        let endianness = match chars.next() {
            Some('<') => Endianness::Little,
            Some('>') => Endianness::Big,
            Some('@') | Some('=') | Some('|') => {
                // Native endianness (resolved at compile time based on target architecture)
                #[cfg(target_endian = "little")]
                {
                    Endianness::Little
                }
                #[cfg(target_endian = "big")]
                {
                    Endianness::Big
                }
            }
            Some(c) => {
                return Err(PyValueError::new_err(format!(
                    "invalid endianness character: '{c}'"
                )))
            }
            None => return Err(PyValueError::new_err("empty format string".to_string())),
        };

        let dtype = match chars.next() {
            Some('B') => DataType::UInt8,
            Some('H') => DataType::UInt16,
            Some('I') => DataType::UInt32,
            Some('Q') => DataType::UInt64,
            Some('b') => DataType::Int8,
            Some('h') => DataType::Int16,
            Some('i') => DataType::Int32,
            Some('q') => DataType::Int64,
            Some('f') => DataType::Float32,
            Some('d') => DataType::Float64,
            Some(c) => {
                return Err(PyValueError::new_err(format!(
                    "invalid type character: '{c}'"
                )))
            }
            None => {
                return Err(PyValueError::new_err(
                    "missing type character after endianness".to_string(),
                ))
            }
        };

        if chars.next().is_some() {
            return Err(PyValueError::new_err(format!(
                "unexpected characters after format: '{s}'"
            )));
        }

        Ok(DataTypeAndEndianness { dtype, endianness })
    }
}

/// Formats as a numpy dtype string (e.g., "<u2", ">f4").
///
/// Note: This is different from the buffer protocol format string!
/// - Numpy dtype: "<u2" (endianness + type char + byte size)
/// - Buffer protocol: "<H" (endianness + struct format char)
impl Display for DataTypeAndEndianness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}",
            self.endianness.char(),
            self.dtype.numpy_char(),
            self.dtype.size()
        )
    }
}

/// A 3D array that implements Python's buffer protocol.
///
/// This allows zero-copy interoperability with numpy via `np.asarray(arr)`.
/// The array is immutable (frozen) and exposes a read-only buffer.
///
/// See `_array.pyi` for Python usage examples and API documentation.
#[pyclass(name = "Array", frozen)]
pub struct PyArray {
    /// The raw byte data backing the array.
    data: Bytes,

    /// The shape of the array as `[dim0, dim1, dim2]`.
    ///
    /// Stored as `isize` because the buffer protocol requires `Py_ssize_t` (= `isize`).
    /// Using `Box` ensures a stable memory address that we can expose to Python.
    ///
    /// The interpretation depends on the PlanarConfiguration:
    /// - PlanarConfiguration=1 (chunky): (height, width, bands)
    /// - PlanarConfiguration=2 (planar): (bands, height, width)
    shape: Box<[isize; 3]>,

    /// Row-major (C-contiguous) strides in bytes.
    ///
    /// For a 3D array with shape [d0, d1, d2] and element size `itemsize`:
    /// - strides[0] = d1 * d2 * itemsize (bytes to skip for next row)
    /// - strides[1] = d2 * itemsize (bytes to skip for next column)
    /// - strides[2] = itemsize (bytes to skip for next element)
    ///
    /// Stored as `Box` for the same reason as `shape`.
    strides: Box<[isize; 3]>,

    /// The data type and endianness of array elements.
    dtype: DataTypeAndEndianness,
}

#[pymethods]
impl PyArray {
    #[new]
    fn new(data: PyBytes, shape: [u32; 3], format: &str) -> PyResult<Self> {
        let dtype: DataTypeAndEndianness = format.parse()?;
        let itemsize = dtype.dtype.size();
        let shape_isize: Box<[isize; 3]> =
            Box::new([shape[0] as isize, shape[1] as isize, shape[2] as isize]);
        // Row-major (C-contiguous) strides: [dim1 * dim2 * itemsize, dim2 * itemsize, itemsize]
        let strides: Box<[isize; 3]> = Box::new([
            (shape[1] as usize * shape[2] as usize * itemsize) as isize,
            (shape[2] as usize * itemsize) as isize,
            itemsize as isize,
        ]);
        Ok(Self {
            data: data.into_inner(),
            shape: shape_isize,
            strides,
            dtype,
        })
    }

    #[getter]
    fn shape(&self) -> [isize; 3] {
        *self.shape
    }

    #[getter]
    fn buffer(&self) -> PyBytes {
        PyBytes::new(self.data.clone())
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
        let itemsize = slf.dtype.dtype.size();

        // Fill in the Py_buffer struct fields
        // SAFETY: view is a valid pointer provided by Python's buffer protocol machinery
        (*view).buf = slf.data.as_ptr() as *mut std::ffi::c_void;
        (*view).len = slf.data.len() as isize;
        (*view).itemsize = itemsize as isize;
        (*view).readonly = 1; // Read-only buffer
        (*view).ndim = 3;

        // Only provide format string if requested (PyBUF_FORMAT flag)
        (*view).format = if flags & ffi::PyBUF_FORMAT != 0 {
            // SAFETY: buffer_format() returns a pointer to a static CStr
            slf.dtype.buffer_format().as_ptr() as *mut std::ffi::c_char
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
