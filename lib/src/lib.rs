#![doc = include_str!("../../README.md")]
#![warn(missing_docs)]

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
mod tiff;
mod tile;

pub use ifd::ImageFileDirectory;
pub use tag_value::TagValue;
pub use tiff::TIFF;
pub use tile::Tile;
