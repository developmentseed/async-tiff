use std::fmt;

use anyhow::Result;
use async_tiff::TIFF;
use serde::Serialize;

#[derive(Serialize)]
pub struct TiffInfo {
    endianness: String,
    num_ifds: usize,
    ifds: Vec<IfdInfo>,
}

#[derive(Serialize)]
struct IfdInfo {
    image_width: u32,
    image_height: u32,
    samples_per_pixel: u16,
    bits_per_sample: Vec<u16>,
    compression: String,
    photometric_interpretation: String,
    tile_count: Option<(usize, usize)>,
    tile_width: Option<u32>,
    tile_height: Option<u32>,
    other_tags_count: usize,
}

impl TiffInfo {
    pub fn from_tiff(tiff: &TIFF) -> Self {
        Self {
            endianness: format!("{:?}", tiff.endianness()),
            num_ifds: tiff.ifds().len(),
            ifds: tiff.ifds().iter().map(IfdInfo::from_ifd).collect(),
        }
    }
}

impl IfdInfo {
    fn from_ifd(ifd: &async_tiff::ImageFileDirectory) -> Self {
        Self {
            image_width: ifd.image_width(),
            image_height: ifd.image_height(),
            samples_per_pixel: ifd.samples_per_pixel(),
            bits_per_sample: ifd.bits_per_sample().to_vec(),
            compression: format!("{:?}", ifd.compression()),
            photometric_interpretation: format!("{:?}", ifd.photometric_interpretation()),
            tile_count: ifd.tile_count(),
            tile_width: ifd.tile_width(),
            tile_height: ifd.tile_height(),
            other_tags_count: ifd.other_tags().len(),
        }
    }
}

impl fmt::Display for TiffInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Endianness: {}", self.endianness)?;
        writeln!(f, "Number of IFDs: {}", self.num_ifds)?;

        for (idx, ifd) in self.ifds.iter().enumerate() {
            writeln!(f, "\nIFD {}:", idx)?;
            writeln!(f, "  Image dimensions: {}x{}", ifd.image_width, ifd.image_height)?;
            writeln!(f, "  Samples per pixel: {}", ifd.samples_per_pixel)?;
            writeln!(f, "  Bits per sample: {:?}", ifd.bits_per_sample)?;
            writeln!(f, "  Compression: {}", ifd.compression)?;
            writeln!(f, "  Photometric interpretation: {}", ifd.photometric_interpretation)?;

            if let Some((x_tiles, y_tiles)) = ifd.tile_count {
                writeln!(f, "  Tiled: {}x{} tiles", x_tiles, y_tiles)?;
                if let Some(tile_width) = ifd.tile_width {
                    writeln!(f, "    Tile width: {}", tile_width)?;
                }
                if let Some(tile_height) = ifd.tile_height {
                    writeln!(f, "    Tile height: {}", tile_height)?;
                }
            } else {
                writeln!(f, "  Striped image")?;
            }

            writeln!(f, "  Other tags: {}", ifd.other_tags_count)?;
        }
        Ok(())
    }
}

pub enum OutputFormat {
    Text,
    Json,
}

pub async fn execute(tiff: &TIFF, format: OutputFormat) -> Result<()> {
    let info = TiffInfo::from_tiff(tiff);

    match format {
        OutputFormat::Text => println!("{}", info),
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&info)?),
    }

    Ok(())
}
