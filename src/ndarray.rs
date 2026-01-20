//! [`ndarray`] integration for async-tiff

use bytemuck::cast_slice;
use ndarray::{ArrayView3, CowArray, Ix3};

use crate::error::AsyncTiffError;
use crate::{Array, DataType};

/// An enum representing a view of a 3D ndarray with various possible data types.
///
/// Note: We use CowArray because whether we can expose data as zero-copy depends on whether the
/// endianness of the TIFF data matches the host system. If it doesn't match, we need to allocate a
/// new array with the correct endianness.
pub enum NdArrayView<'a> {
    /// Unsigned 8-bit integer array
    Uint8(CowArray<'a, u8, Ix3>),

    /// Unsigned 16-bit integer array
    Uint16(CowArray<'a, u16, Ix3>),

    /// Unsigned 32-bit integer array
    Uint32(CowArray<'a, u32, Ix3>),

    /// Unsigned 64-bit integer array
    Uint64(CowArray<'a, u64, Ix3>),

    /// Signed 8-bit integer array
    Int8(CowArray<'a, i8, Ix3>),

    /// Signed 16-bit integer array
    Int16(CowArray<'a, i16, Ix3>),

    /// Signed 32-bit integer array
    Int32(CowArray<'a, i32, Ix3>),

    /// Signed 64-bit integer array
    Int64(CowArray<'a, i64, Ix3>),

    /// 32-bit floating point array
    Float32(CowArray<'a, f32, Ix3>),

    /// 64-bit floating point array
    Float64(CowArray<'a, f64, Ix3>),
}

impl<'a> TryFrom<&'a Array> for NdArrayView<'a> {
    type Error = crate::error::AsyncTiffError;

    fn try_from(value: &'a Array) -> Result<Self, Self::Error> {
        if !value.endianness().is_native() {
            // We'll have to copy array data and convert endianness
            todo!("Handle non-native endianness conversions for ndarray integration");
        }

        let data_type = value
            .data_type
            .ok_or_else(|| AsyncTiffError::General("Unknown data type".to_string()))?;
        match data_type {
            DataType::UInt8 => {
                let view = ArrayView3::from_shape(value.shape, value.data.as_ref()).unwrap();
                Ok(NdArrayView::Uint8(CowArray::from(view)))
            }
            DataType::UInt16 => {
                let view =
                    ArrayView3::from_shape(value.shape, cast_slice(value.data.as_ref())).unwrap();
                Ok(NdArrayView::Uint16(CowArray::from(view)))
            }
            DataType::UInt32 => {
                let view =
                    ArrayView3::from_shape(value.shape, cast_slice(value.data.as_ref())).unwrap();
                Ok(NdArrayView::Uint32(CowArray::from(view)))
            }
            DataType::UInt64 => {
                let view =
                    ArrayView3::from_shape(value.shape, cast_slice(value.data.as_ref())).unwrap();
                Ok(NdArrayView::Uint64(CowArray::from(view)))
            }
            DataType::Int8 => {
                let view =
                    ArrayView3::from_shape(value.shape, cast_slice(value.data.as_ref())).unwrap();
                Ok(NdArrayView::Int8(CowArray::from(view)))
            }
            DataType::Int16 => {
                let view =
                    ArrayView3::from_shape(value.shape, cast_slice(value.data.as_ref())).unwrap();
                Ok(NdArrayView::Int16(CowArray::from(view)))
            }
            DataType::Int32 => {
                let view =
                    ArrayView3::from_shape(value.shape, cast_slice(value.data.as_ref())).unwrap();
                Ok(NdArrayView::Int32(CowArray::from(view)))
            }
            DataType::Int64 => {
                let view =
                    ArrayView3::from_shape(value.shape, cast_slice(value.data.as_ref())).unwrap();
                Ok(NdArrayView::Int64(CowArray::from(view)))
            }
            DataType::Float32 => {
                let view =
                    ArrayView3::from_shape(value.shape, cast_slice(value.data.as_ref())).unwrap();
                Ok(NdArrayView::Float32(CowArray::from(view)))
            }
            DataType::Float64 => {
                let view =
                    ArrayView3::from_shape(value.shape, cast_slice(value.data.as_ref())).unwrap();
                Ok(NdArrayView::Float64(CowArray::from(view)))
            }
        }
    }
}
