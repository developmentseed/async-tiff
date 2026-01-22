use bytemuck::{cast_slice, cast_vec, try_cast_vec};

use crate::data_type::DataType;
use crate::error::{AsyncTiffError, AsyncTiffResult};

/// A 3D array that represents decoded TIFF image data.
#[derive(Debug, Clone)]
pub struct Array {
    /// The raw byte data of the array.
    pub(crate) data: TypedArray,

    /// The 3D shape of the array.
    ///
    /// The axis ordering depends on the PlanarConfiguration:
    ///
    /// - PlanarConfiguration=1 (chunky): (height, width, bands)
    /// - PlanarConfiguration=2 (planar): (bands, height, width)
    pub(crate) shape: [usize; 3],

    /// The data type of the array elements.
    ///
    /// If None, the data type is unsupported or unknown.
    pub(crate) data_type: Option<DataType>,
}

impl Array {
    pub(crate) fn try_new(
        data: Vec<u8>,
        shape: [usize; 3],
        data_type: Option<DataType>,
    ) -> AsyncTiffResult<Self> {
        // Validate that the data length matches the expected size
        let expected_len = shape[0] * shape[1] * shape[2];

        let typed_data = TypedArray::new(data, data_type);
        if typed_data.len() != expected_len {
            return Err(AsyncTiffError::General(format!("Internal error: incorrect shape or data length passed to Array::try_new. Got data length {}, expected {}", typed_data.len(), expected_len)));
        }

        Ok(Self {
            data: typed_data,
            shape,
            data_type,
        })
    }

    /// Access the raw underlying byte data of the array.
    pub fn data(&self) -> &TypedArray {
        &self.data
    }

    /// Consume the Array and return its components.
    pub fn into_inner(self) -> (TypedArray, [usize; 3], Option<DataType>) {
        (self.data, self.shape, self.data_type)
    }

    /// Get the shape of the array.
    ///
    /// The shape matches the physical array data exposed, but the _interpretation_ depends on the
    /// value of `PlanarConfiguration`:
    ///
    /// - PlanarConfiguration=1 (chunky): (height, width, bands)
    /// - PlanarConfiguration=2 (planar): (bands, height, width)
    pub fn shape(&self) -> [usize; 3] {
        self.shape
    }

    /// The logical data type of the array elements.
    ///
    /// If None, the data type is unsupported or unknown.
    pub fn data_type(&self) -> Option<DataType> {
        self.data_type
    }
}

/// An enum representing a typed view of the array data.
#[derive(Debug, Clone)]
pub enum TypedArray {
    /// Unsigned 8-bit integer array.
    UInt8(Vec<u8>),
    /// Unsigned 16-bit integer array.
    UInt16(Vec<u16>),
    /// Unsigned 32-bit integer array.
    UInt32(Vec<u32>),
    /// Unsigned 64-bit integer array.
    UInt64(Vec<u64>),
    /// Signed 8-bit integer array.
    Int8(Vec<i8>),
    /// Signed 16-bit integer array.
    Int16(Vec<i16>),
    /// Signed 32-bit integer array.
    Int32(Vec<i32>),
    /// Signed 64-bit integer array.
    Int64(Vec<i64>),
    /// 32-bit floating point array.
    Float32(Vec<f32>),
    /// 64-bit floating point array.
    Float64(Vec<f64>),
}

impl TypedArray {
    /// Create a new TypedArray from raw byte data and a specified DataType.
    pub fn new(data: Vec<u8>, data_type: Option<DataType>) -> Self {
        match data_type {
            None | Some(DataType::UInt8) => TypedArray::UInt8(data),
            Some(DataType::UInt16) => {
                TypedArray::UInt16(try_cast_vec(data).unwrap_or_else(|(_, data)| {
                    // Fallback to manual conversion when not aligned
                    data.chunks_exact(2)
                        .map(|b| u16::from_ne_bytes([b[0], b[1]]))
                        .collect()
                }))
            }
            Some(DataType::UInt32) => {
                TypedArray::UInt32(try_cast_vec(data).unwrap_or_else(|(_, data)| {
                    // Fallback to manual conversion when not aligned
                    data.chunks_exact(4)
                        .map(|b| u32::from_ne_bytes([b[0], b[1], b[2], b[3]]))
                        .collect()
                }))
            }
            Some(DataType::UInt64) => {
                TypedArray::UInt64(try_cast_vec(data).unwrap_or_else(|(_, data)| {
                    // Fallback to manual conversion when not aligned
                    data.chunks_exact(8)
                        .map(|b| {
                            u64::from_ne_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
                        })
                        .collect()
                }))
            }
            // Casting u8 to i8 is safe as they have the same memory representation
            Some(DataType::Int8) => TypedArray::Int8(cast_vec(data)),
            Some(DataType::Int16) => {
                TypedArray::Int16(try_cast_vec(data).unwrap_or_else(|(_, data)| {
                    // Fallback to manual conversion when not aligned
                    data.chunks_exact(2)
                        .map(|b| i16::from_ne_bytes([b[0], b[1]]))
                        .collect()
                }))
            }
            Some(DataType::Int32) => {
                TypedArray::Int32(try_cast_vec(data).unwrap_or_else(|(_, data)| {
                    // Fallback to manual conversion when not aligned
                    data.chunks_exact(4)
                        .map(|b| i32::from_ne_bytes([b[0], b[1], b[2], b[3]]))
                        .collect()
                }))
            }
            Some(DataType::Int64) => {
                TypedArray::Int64(try_cast_vec(data).unwrap_or_else(|(_, data)| {
                    // Fallback to manual conversion when not aligned
                    data.chunks_exact(8)
                        .map(|b| {
                            i64::from_ne_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
                        })
                        .collect()
                }))
            }
            Some(DataType::Float32) => {
                TypedArray::Float32(try_cast_vec(data).unwrap_or_else(|(_, data)| {
                    // Fallback to manual conversion when not aligned
                    data.chunks_exact(4)
                        .map(|b| f32::from_ne_bytes([b[0], b[1], b[2], b[3]]))
                        .collect()
                }))
            }
            Some(DataType::Float64) => {
                TypedArray::Float64(try_cast_vec(data).unwrap_or_else(|(_, data)| {
                    // Fallback to manual conversion when not aligned
                    data.chunks_exact(8)
                        .map(|b| {
                            f64::from_ne_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
                        })
                        .collect()
                }))
            }
        }
    }

    /// Get the length (number of elements) of the typed array.
    pub fn len(&self) -> usize {
        match self {
            TypedArray::UInt8(data) => data.len(),
            TypedArray::UInt16(data) => data.len(),
            TypedArray::UInt32(data) => data.len(),
            TypedArray::UInt64(data) => data.len(),
            TypedArray::Int8(data) => data.len(),
            TypedArray::Int16(data) => data.len(),
            TypedArray::Int32(data) => data.len(),
            TypedArray::Int64(data) => data.len(),
            TypedArray::Float32(data) => data.len(),
            TypedArray::Float64(data) => data.len(),
        }
    }

    /// Check if the typed array is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl AsRef<[u8]> for TypedArray {
    fn as_ref(&self) -> &[u8] {
        match self {
            TypedArray::UInt8(data) => data.as_slice(),
            TypedArray::UInt16(data) => cast_slice(data),
            TypedArray::UInt32(data) => cast_slice(data),
            TypedArray::UInt64(data) => cast_slice(data),
            TypedArray::Int8(data) => cast_slice(data),
            TypedArray::Int16(data) => cast_slice(data),
            TypedArray::Int32(data) => cast_slice(data),
            TypedArray::Int64(data) => cast_slice(data),
            TypedArray::Float32(data) => cast_slice(data),
            TypedArray::Float64(data) => cast_slice(data),
        }
    }
}
