#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(not(test), deny(unused_crate_dependencies))]
#![doc(
    html_logo_url = "https://github.com/developmentseed.png",
    html_favicon_url = "https://github.com/developmentseed.png?size=32"
)]

mod array;
mod data_type;
pub mod decoder;
pub mod error;
pub mod geo;
mod ifd;
pub mod metadata;
#[cfg(feature = "ndarray")]
pub mod ndarray;
mod predictor;
pub mod reader;
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
