#![doc = include_str!("../README.md")]

mod async_reader;
mod cog;
mod decoder;
pub mod error;
pub mod geo;
mod ifd;

pub use async_reader::{AsyncFileReader, ObjectReader};
pub use cog::COGReader;
pub use ifd::{ImageFileDirectories, ImageFileDirectory};
