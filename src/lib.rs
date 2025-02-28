#![doc = include_str!("../README.md")]

mod async_reader;
mod cog;
pub mod decoder;
pub mod error;
pub mod geo;
mod ifd;
pub mod tiff;
mod tile;

pub use async_reader::{AsyncFileReader, ObjectReader, PrefetchReader};
pub use cog::COGReader;
pub use ifd::{ImageFileDirectories, ImageFileDirectory};
pub use tile::Tile;
