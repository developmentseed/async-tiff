use bytes::Bytes;

use crate::decoder::DecoderRegistry;
use crate::error::AsyncTiffResult;
use crate::predictor::{fix_endianness, unpredict_float, unpredict_hdiff, PredictorInfo};
use crate::tiff::tags::{CompressionMethod, PhotometricInterpretation, Predictor};
use crate::tiff::{TiffError, TiffUnsupportedError};

/// A TIFF Tile response.
///
/// This contains the required information to decode the tile. Decoding is separated from fetching
/// so that sync and async operations can be separated and non-blocking.
///
/// This is returned by `fetch_tile`.
///
/// A strip of a stripped tiff is an image-width, rows-per-strip tile.
#[derive(Debug)]
pub struct Tile {
    pub(crate) x: usize,
    pub(crate) y: usize,
    pub(crate) predictor: Predictor,
    pub(crate) predictor_info: PredictorInfo,
    pub(crate) compressed_bytes: Bytes,
    pub(crate) compression_method: CompressionMethod,
    pub(crate) photometric_interpretation: PhotometricInterpretation,
    pub(crate) jpeg_tables: Option<Bytes>,
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
    pub fn compressed_bytes(&self) -> &Bytes {
        &self.compressed_bytes
    }

    /// Access the compression tag representing this tile.
    pub fn compression_method(&self) -> CompressionMethod {
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

    /// Decode this tile.
    ///
    /// Decoding is separate from fetching so that sync and async operations do not block the same
    /// runtime.
    pub fn decode(self, decoder_registry: &DecoderRegistry) -> AsyncTiffResult<Bytes> {
        let decoder = decoder_registry
            .as_ref()
            .get(&self.compression_method)
            .ok_or(TiffError::UnsupportedError(
                TiffUnsupportedError::UnsupportedCompressionMethod(self.compression_method),
            ))?;

        let decoded_tile = decoder.decode_tile(
            self.compressed_bytes.clone(),
            self.photometric_interpretation,
            self.jpeg_tables.as_deref(),
        )?;

        match self.predictor {
            Predictor::None => Ok(fix_endianness(
                decoded_tile,
                self.predictor_info.endianness(),
                self.predictor_info.bits_per_sample(),
            )),
            Predictor::Horizontal => {
                unpredict_hdiff(decoded_tile, &self.predictor_info, self.x as _)
            }
            Predictor::FloatingPoint => {
                unpredict_float(decoded_tile, &self.predictor_info, self.x as _, self.y as _)
            }
        }
    }
}
