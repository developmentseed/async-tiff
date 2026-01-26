#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

mod array;
mod data_type;
mod extension;
#[cfg(feature = "ndarray")]
pub mod ndarray;
pub mod reader;
// TODO: maybe rename this mod
pub mod decoder;
pub mod error;
pub mod geo;
mod ifd;
pub mod metadata;
pub mod predictor;
mod tag_value;
pub mod tags;
#[cfg(test)]
mod test;
mod tiff;
mod tile;

pub use array::{Array, TypedArray};
pub use data_type::DataType;
pub use ifd::ImageFileDirectory;
pub use tag_value::TagValue;
pub use tiff::TIFF;
pub use tile::Tile;
