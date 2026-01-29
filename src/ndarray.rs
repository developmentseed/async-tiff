//! [`ndarray`] integration for async-tiff.
//!
//! Use [`Tile::decode`][crate::Tile::decode] to decode a tile into an [`Array`], then use the
//! `TryFrom<Array>` implementation on [`NdArray`] for easier manipulation with the [`ndarray`]
//! crate.

use ndarray::Array3;

use crate::error::AsyncTiffError;
use crate::{Array, TypedArray};

/// An enum representing a view of a [`ndarray::Array3`] with various possible data types.
pub enum NdArray {
    /// Unsigned 8-bit integer array
    Uint8(Array3<u8>),

    /// Unsigned 16-bit integer array
    Uint16(Array3<u16>),

    /// Unsigned 32-bit integer array
    Uint32(Array3<u32>),

    /// Unsigned 64-bit integer array
    Uint64(Array3<u64>),

    /// Signed 8-bit integer array
    Int8(Array3<i8>),

    /// Signed 16-bit integer array
    Int16(Array3<i16>),

    /// Signed 32-bit integer array
    Int32(Array3<i32>),

    /// Signed 64-bit integer array
    Int64(Array3<i64>),

    /// 32-bit floating point array
    Float32(Array3<f32>),

    /// 64-bit floating point array
    Float64(Array3<f64>),
}

impl TryFrom<Array> for NdArray {
    type Error = crate::error::AsyncTiffError;

    fn try_from(value: Array) -> Result<Self, Self::Error> {
        // Check for unsupported data type
        value
            .data_type
            .ok_or_else(|| AsyncTiffError::General("Unknown data type".to_string()))?;
        match value.data {
            TypedArray::BitMask { .. } => Err(AsyncTiffError::General(
                "BitMask arrays cannot be converted to ndarray (sub-byte elements not supported)"
                    .to_string(),
            )),
            TypedArray::UInt8(data) => Ok(NdArray::Uint8(
                Array3::from_shape_vec(value.shape, data).map_err(|e| {
                    AsyncTiffError::General(format!("Failed to create ndarray: {}", e))
                })?,
            )),
            TypedArray::UInt16(data) => Ok(NdArray::Uint16(
                Array3::from_shape_vec(value.shape, data).map_err(|e| {
                    AsyncTiffError::General(format!("Failed to create ndarray: {}", e))
                })?,
            )),
            TypedArray::UInt32(data) => Ok(NdArray::Uint32(
                Array3::from_shape_vec(value.shape, data).map_err(|e| {
                    AsyncTiffError::General(format!("Failed to create ndarray: {}", e))
                })?,
            )),
            TypedArray::UInt64(data) => Ok(NdArray::Uint64(
                Array3::from_shape_vec(value.shape, data).map_err(|e| {
                    AsyncTiffError::General(format!("Failed to create ndarray: {}", e))
                })?,
            )),
            TypedArray::Int8(data) => Ok(NdArray::Int8(
                Array3::from_shape_vec(value.shape, data).map_err(|e| {
                    AsyncTiffError::General(format!("Failed to create ndarray: {}", e))
                })?,
            )),
            TypedArray::Int16(data) => Ok(NdArray::Int16(
                Array3::from_shape_vec(value.shape, data).map_err(|e| {
                    AsyncTiffError::General(format!("Failed to create ndarray: {}", e))
                })?,
            )),
            TypedArray::Int32(data) => Ok(NdArray::Int32(
                Array3::from_shape_vec(value.shape, data).map_err(|e| {
                    AsyncTiffError::General(format!("Failed to create ndarray: {}", e))
                })?,
            )),
            TypedArray::Int64(data) => Ok(NdArray::Int64(
                Array3::from_shape_vec(value.shape, data).map_err(|e| {
                    AsyncTiffError::General(format!("Failed to create ndarray: {}", e))
                })?,
            )),
            TypedArray::Float32(data) => Ok(NdArray::Float32(
                Array3::from_shape_vec(value.shape, data).map_err(|e| {
                    AsyncTiffError::General(format!("Failed to create ndarray: {}", e))
                })?,
            )),
            TypedArray::Float64(data) => Ok(NdArray::Float64(
                Array3::from_shape_vec(value.shape, data).map_err(|e| {
                    AsyncTiffError::General(format!("Failed to create ndarray: {}", e))
                })?,
            )),
        }
    }
}
