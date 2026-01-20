use bytemuck::cast_slice;
use bytes::Bytes;

use crate::data_type::DataType;
use crate::reader::Endianness;

/// A 3D array that represents decoded TIFF image data.
#[derive(Debug, Clone)]
pub struct Array {
    /// The raw byte data of the array.
    pub(crate) data: Bytes,

    /// The endianness of the data.
    pub(crate) endianness: Endianness,

    /// The shape of the array as [height, width, channels].
    ///
    /// The interpretation depends on the PlanarConfiguration:
    /// - PlanarConfiguration=1 (chunky): (height, width, bands)
    /// - PlanarConfiguration=2 (planar): (bands, height, width)
    pub(crate) shape: [usize; 3],

    /// The data type of the array elements.
    ///
    /// If None, the data type is unsupported or unknown.
    pub(crate) data_type: Option<DataType>,
}

impl Array {
    pub(crate) fn new(
        data: Bytes,
        endianness: Endianness,
        shape: [usize; 3],
        data_type: Option<DataType>,
    ) -> Self {
        Self {
            data,
            endianness,
            shape,
            data_type,
        }
    }

    /// Access the raw underlying byte data of the array.
    ///
    /// Use [`as_typed`][Self::as_typed] to get a typed view of the data.
    pub fn raw_data(&self) -> &Bytes {
        &self.data
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

    /// Get the endianness of the array data.
    pub fn endianness(&self) -> Endianness {
        self.endianness
    }

    /// The logical data type of the array elements.
    ///
    /// If None, the data type is unsupported or unknown.
    pub fn data_type(&self) -> Option<DataType> {
        self.data_type
    }

    /// Get a typed view of the array data.
    pub fn as_typed(&self) -> Option<TypedArray<'_>> {
        match self.data_type? {
            DataType::UInt8 => Some(TypedArray::Uint8(&self.data)),
            DataType::UInt16 => Some(TypedArray::Uint16(cast_slice(&self.data))),
            DataType::UInt32 => Some(TypedArray::Uint32(cast_slice(&self.data))),
            DataType::UInt64 => Some(TypedArray::Uint64(cast_slice(&self.data))),
            DataType::Int8 => Some(TypedArray::Int8(cast_slice(&self.data))),
            DataType::Int16 => Some(TypedArray::Int16(cast_slice(&self.data))),
            DataType::Int32 => Some(TypedArray::Int32(cast_slice(&self.data))),
            DataType::Int64 => Some(TypedArray::Int64(cast_slice(&self.data))),
            DataType::Float32 => Some(TypedArray::Float32(cast_slice(&self.data))),
            DataType::Float64 => Some(TypedArray::Float64(cast_slice(&self.data))),
        }
    }
}

/// An enum representing a typed view of the array data.
#[derive(Debug, Clone, Copy)]
pub enum TypedArray<'a> {
    /// Unsigned 8-bit integer array.
    Uint8(&'a [u8]),
    /// Unsigned 16-bit integer array.
    Uint16(&'a [u16]),
    /// Unsigned 32-bit integer array.
    Uint32(&'a [u32]),
    /// Unsigned 64-bit integer array.
    Uint64(&'a [u64]),
    /// Signed 8-bit integer array.
    Int8(&'a [i8]),
    /// Signed 16-bit integer array.
    Int16(&'a [i16]),
    /// Signed 32-bit integer array.
    Int32(&'a [i32]),
    /// Signed 64-bit integer array.
    Int64(&'a [i64]),
    /// 32-bit floating point array.
    Float32(&'a [f32]),
    /// 64-bit floating point array.
    Float64(&'a [f64]),
}
