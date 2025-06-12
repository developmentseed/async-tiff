//! Decoders for different TIFF compression methods.

use std::collections::HashMap;
use std::fmt::Debug;
use std::io::{Cursor, Read};

use bytes::Bytes;
use flate2::bufread::ZlibDecoder;

use crate::error::{AsyncTiffResult, AsyncTiffError};
use crate::tiff::tags::{CompressionMethod, PhotometricInterpretation};
use crate::tiff::{TiffError, TiffUnsupportedError};

/// A registry of decoders.
///
/// This allows end users to register their own decoders, for custom compression methods, or
/// override the default decoder implementations.
#[derive(Debug)]
pub struct DecoderRegistry(HashMap<CompressionMethod, Box<dyn Decoder>>);

impl DecoderRegistry {
    /// Create a new decoder registry with no decoders registered
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

impl AsRef<HashMap<CompressionMethod, Box<dyn Decoder>>> for DecoderRegistry {
    fn as_ref(&self) -> &HashMap<CompressionMethod, Box<dyn Decoder>> {
        &self.0
    }
}

impl AsMut<HashMap<CompressionMethod, Box<dyn Decoder>>> for DecoderRegistry {
    fn as_mut(&mut self) -> &mut HashMap<CompressionMethod, Box<dyn Decoder>> {
        &mut self.0
    }
}

impl Default for DecoderRegistry {
    fn default() -> Self {
        let mut registry = HashMap::with_capacity(5);
        registry.insert(CompressionMethod::None, Box::new(UncompressedDecoder) as _);
        registry.insert(CompressionMethod::Deflate, Box::new(DeflateDecoder) as _);
        registry.insert(CompressionMethod::OldDeflate, Box::new(DeflateDecoder) as _);
        registry.insert(CompressionMethod::LZW, Box::new(LZWDecoder) as _);
        registry.insert(CompressionMethod::ModernJPEG, Box::new(JPEGDecoder) as _);
        Self(registry)
    }
}

/// A trait to decode a TIFF tile.
pub trait Decoder: Debug + Send + Sync {
    /// Decode a TIFF tile.
    fn decode_tile(
        &self,
        compressed_buffer: Bytes,
        result_buffer: &mut [u8],
        photometric_interpretation: PhotometricInterpretation,
        jpeg_tables: Option<&[u8]>,
    ) -> AsyncTiffResult<()>;
}

/// A decoder for the Deflate compression method.
#[derive(Debug, Clone)]
pub struct DeflateDecoder;

impl Decoder for DeflateDecoder {
    fn decode_tile(
        &self,
        compressed_buffer: Bytes,
        result_buffer: &mut [u8],
        _photometric_interpretation: PhotometricInterpretation,
        _jpeg_tables: Option<&[u8]>,
    ) -> AsyncTiffResult<()> {
        let mut decoder = ZlibDecoder::new(Cursor::new(compressed_buffer));
        decoder.read_exact(result_buffer)?;
        Ok(())
    }
}

/// A decoder for the JPEG compression method.
#[derive(Debug, Clone)]
pub struct JPEGDecoder;

impl Decoder for JPEGDecoder {
    fn decode_tile(
        &self,
        compressed_buffer: Bytes,
        result_buffer: &mut [u8],
        photometric_interpretation: PhotometricInterpretation,
        jpeg_tables: Option<&[u8]>,
    ) -> AsyncTiffResult<()> {
        decode_modern_jpeg(
            compressed_buffer,
            result_buffer,
            photometric_interpretation,
            jpeg_tables,
        )
    }
}

/// A decoder for the LZW compression method.
#[derive(Debug, Clone)]
pub struct LZWDecoder;

impl Decoder for LZWDecoder {
    fn decode_tile(
        &self,
        compressed_buffer: Bytes,
        result_buffer: &mut [u8],
        _photometric_interpretation: PhotometricInterpretation,
        _jpeg_tables: Option<&[u8]>,
    ) -> AsyncTiffResult<()> {
        // https://github.com/image-rs/image-tiff/blob/90ae5b8e54356a35e266fb24e969aafbcb26e990/src/decoder/stream.rs#L147
        let mut decoder = weezl::decode::Decoder::with_tiff_size_switch(weezl::BitOrder::Msb, 8);
        let buf_res = decoder
            .decode_bytes(&compressed_buffer, result_buffer);
        match buf_res.status {
            Err(e) => Err(AsyncTiffError::External(Box::new(e))),
            Ok(lzw_status) => match lzw_status {
                weezl::LzwStatus::Ok | weezl::LzwStatus::Done => Ok(()),
                weezl::LzwStatus::NoProgress => Err(AsyncTiffError::General("Internal LZW decoder reported no progress".into()))
            }
        }
    }
}

/// A decoder for uncompressed data.
#[derive(Debug, Clone)]
pub struct UncompressedDecoder;

impl Decoder for UncompressedDecoder {
    fn decode_tile(
        &self,
        compressed_buffer: Bytes,
        result_buffer: &mut [u8],
        _photometric_interpretation: PhotometricInterpretation,
        _jpeg_tables: Option<&[u8]>,
    ) -> AsyncTiffResult<()> {
        assert_eq!(compressed_buffer.len(), result_buffer.len());
        // we still need to copy into the typed array
        result_buffer.copy_from_slice(&compressed_buffer);
        Ok(())
    }
}

// https://github.com/image-rs/image-tiff/blob/3bfb43e83e31b0da476832067ada68a82b378b7b/src/decoder/image.rs#L389-L450
fn decode_modern_jpeg(
    compressed_buffer: Bytes,
    result_buffer: &mut [u8],
    photometric_interpretation: PhotometricInterpretation,
    jpeg_tables: Option<&[u8]>,
) -> AsyncTiffResult<()> {
    // Construct new jpeg_reader wrapping a SmartReader.
    //
    // JPEG compression in TIFF allows saving quantization and/or huffman tables in one central
    // location. These `jpeg_tables` are simply prepended to the remaining jpeg image data. Because
    // these `jpeg_tables` start with a `SOI` (HEX: `0xFFD8`) or __start of image__ marker which is
    // also at the beginning of the remaining JPEG image data and would confuse the JPEG renderer,
    // one of these has to be taken off. In this case the first two bytes of the remaining JPEG
    // data is removed because it follows `jpeg_tables`. Similary, `jpeg_tables` ends with a `EOI`
    // (HEX: `0xFFD9`) or __end of image__ marker, this has to be removed as well (last two bytes
    // of `jpeg_tables`).
    let reader = Cursor::new(compressed_buffer);

    let jpeg_reader = match jpeg_tables {
        Some(jpeg_tables) => {
            let mut reader = reader;
            reader.read_exact(&mut [0; 2])?;

            Box::new(Cursor::new(&jpeg_tables[..jpeg_tables.len() - 2]).chain(reader))
                as Box<dyn Read>
        }
        None => Box::new(reader),
    };

    let mut decoder = jpeg::Decoder::new(jpeg_reader);

    match photometric_interpretation {
        PhotometricInterpretation::RGB => decoder.set_color_transform(jpeg::ColorTransform::RGB),
        PhotometricInterpretation::WhiteIsZero
        | PhotometricInterpretation::BlackIsZero
        | PhotometricInterpretation::TransparencyMask => {
            decoder.set_color_transform(jpeg::ColorTransform::None)
        }
        PhotometricInterpretation::CMYK => decoder.set_color_transform(jpeg::ColorTransform::CMYK),
        PhotometricInterpretation::YCbCr => {
            decoder.set_color_transform(jpeg::ColorTransform::YCbCr)
        }
        photometric_interpretation => {
            return Err(TiffError::UnsupportedError(
                TiffUnsupportedError::UnsupportedInterpretation(photometric_interpretation),
            )
            .into());
        }
    }

    let data = decoder.decode()?;
    // jpeg decoder doesn't support decoding into a buffer -> copy
    result_buffer.copy_from_slice(&data);
    Ok(())
}
