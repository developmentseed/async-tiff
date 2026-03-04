use bytes::Bytes;

use crate::array::Array;
use crate::decoder::DecoderRegistry;
use crate::error::{AsyncTiffResult, TiffError, TiffUnsupportedError};
use crate::ifd::CompressedBytes;
use crate::predictor::{fix_endianness, unpredict_float, unpredict_hdiff};
use crate::reader::Endianness;
use crate::tags::{Compression, PhotometricInterpretation, PlanarConfiguration, Predictor};
use crate::DataType;

/// A TIFF Tile response.
///
/// This contains the required information to decode the tile. Decoding is separated from fetching
/// so that sync and async operations can be separated and non-blocking.
///
/// This is returned by `fetch_tile`.
///
/// A strip of a stripped tiff is an image-width, rows-per-strip tile.
#[derive(Debug, Clone)]
pub struct Tile {
    pub(crate) x: usize,
    pub(crate) y: usize,
    pub(crate) data_type: Option<DataType>,
    pub(crate) samples_per_pixel: u16,
    pub(crate) bits_per_sample: u16,
    pub(crate) endianness: Endianness,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) planar_configuration: PlanarConfiguration,
    pub(crate) predictor: Predictor,
    pub(crate) compressed_bytes: CompressedBytes,
    pub(crate) compression_method: Compression,
    pub(crate) photometric_interpretation: PhotometricInterpretation,
    pub(crate) jpeg_tables: Option<Bytes>,
    /// LERC parameters from the LercParameters tag: [version, compression_type, ...]
    /// compression_type: 0 = none, 1 = deflate, 2 = zstd
    pub(crate) lerc_parameters: Option<Vec<u32>>,
}

impl Tile {
    /// The column index of this tile.
    pub fn x(&self) -> usize {
        self.x
    }

    /// The row index of this tile.
    pub fn y(&self) -> usize {
        self.y
    }

    /// Access the compressed bytes underlying this tile.
    ///
    /// Note that [`Bytes`] is reference-counted, so it is very cheap to clone if needed.
    pub fn compressed_bytes(&self) -> &CompressedBytes {
        &self.compressed_bytes
    }

    /// Access the compression tag representing this tile.
    pub fn compression_method(&self) -> Compression {
        self.compression_method
    }

    /// Access the photometric interpretation tag representing this tile.
    pub fn photometric_interpretation(&self) -> PhotometricInterpretation {
        self.photometric_interpretation
    }

    /// Access the JPEG Tables, if any, from the IFD producing this tile.
    ///
    /// Note that [`Bytes`] is reference-counted, so it is very cheap to clone if needed.
    pub fn jpeg_tables(&self) -> Option<&Bytes> {
        self.jpeg_tables.as_ref()
    }

    /// Decode this tile to an [`Array`].
    ///
    /// Decoding is separate from data fetching so that sync and async operations do not block the
    /// same runtime.
    pub fn decode(self, decoder_registry: &DecoderRegistry) -> AsyncTiffResult<Array> {
        let decoder = decoder_registry
            .as_ref()
            .get(&self.compression_method)
            .ok_or(TiffError::UnsupportedError(
                TiffUnsupportedError::UnsupportedCompression(self.compression_method),
            ))?;

        let samples = self.samples_per_pixel as usize;
        let bits_per_sample = self.bits_per_sample;
        // tile_width is the full encoded tile width — predictor must use this, not the cropped width
        let tile_width = self.width as usize;

        let mut decoded_tile = match &self.compressed_bytes {
            CompressedBytes::Chunky(bytes) => decoder.decode_tile(
                bytes.clone(),
                self.photometric_interpretation,
                self.jpeg_tables.as_deref(),
                self.samples_per_pixel,
                bits_per_sample,
                self.lerc_parameters.as_deref(),
            )?,
            CompressedBytes::Planar(band_bytes) => {
                let bytes_per_sample = (bits_per_sample as usize).div_ceil(8);
                let total_size =
                    band_bytes.len() * tile_width * (self.height as usize) * bytes_per_sample;
                let mut result = Vec::with_capacity(total_size);

                for band_data in band_bytes {
                    let decoded_band = decoder.decode_tile(
                        band_data.clone(),
                        self.photometric_interpretation,
                        self.jpeg_tables.as_deref(),
                        1,
                        bits_per_sample,
                        self.lerc_parameters.as_deref(),
                    )?;
                    result.extend_from_slice(&decoded_band);
                }

                debug_assert_eq!(result.len(), total_size);
                result
            }
        };

        // Apply predictor on the full encoded tile width, then crop afterward.
        let decoded = match self.predictor {
            Predictor::None => {
                fix_endianness(&mut decoded_tile, self.endianness, bits_per_sample);
                decoded_tile
            }
            Predictor::Horizontal => unpredict_hdiff(
                decoded_tile,
                self.endianness,
                samples,
                bits_per_sample,
                tile_width,
            ),
            Predictor::FloatingPoint => unpredict_float(
                decoded_tile,
                samples,
                bits_per_sample,
                tile_width,
            )?,
        };

        let shape = infer_shape(
            self.planar_configuration,
            self.width as _,
            self.height as _,
            samples,
        );
        Array::try_new(decoded, shape, self.data_type)
    }
}

fn infer_shape(
    planar_configuration: PlanarConfiguration,
    width: usize,
    height: usize,
    samples_per_pixel: usize,
) -> [usize; 3] {
    match planar_configuration {
        PlanarConfiguration::Chunky => [height, width, samples_per_pixel],
        PlanarConfiguration::Planar => [samples_per_pixel, height, width],
    }
}
