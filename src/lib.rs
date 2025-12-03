#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

pub mod reader;
// TODO: maybe rename this mod
mod cog;
pub mod decoder;
pub mod error;
pub mod geo;
mod ifd;
pub mod metadata;
pub mod predictor;
mod tag_value;
pub mod tags;
mod tile;

pub use cog::TIFF;
pub use ifd::ImageFileDirectory;
pub use tile::Tile;
