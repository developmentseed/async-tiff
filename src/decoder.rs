//! Decoders for different TIFF compression methods.

use std::collections::HashMap;
use std::fmt::Debug;
use std::io::{Cursor, Read};

use bytes::Bytes;
use flate2::bufread::ZlibDecoder;

use crate::error::{AsyncTiffResult, TiffError, TiffUnsupportedError};
use crate::tags::{Compression, PhotometricInterpretation};

/// A registry of decoders.
///
/// This allows end users to register their own decoders, for custom compression methods, or
/// override the default decoder implementations.
///
/// ```
/// use async_tiff::decoder::DecoderRegistry;
///
/// // Default registry includes Deflate, LZW, JPEG, ZSTD.
/// let registry = DecoderRegistry::default();
///
/// // Empty registry for manual configuration.
/// let empty = DecoderRegistry::empty();
/// ```
#[derive(Debug)]
pub struct DecoderRegistry(HashMap<Compression, Box<dyn Decoder>>);

impl DecoderRegistry {
    /// Create a new decoder registry with no decoders registered
    pub fn empty() -> Self {
        Self(HashMap::new())
    }
}

impl AsRef<HashMap<Compression, Box<dyn Decoder>>> for DecoderRegistry {
    fn as_ref(&self) -> &HashMap<Compression, Box<dyn Decoder>> {
        &self.0
    }
}

impl AsMut<HashMap<Compression, Box<dyn Decoder>>> for DecoderRegistry {
    fn as_mut(&mut self) -> &mut HashMap<Compression, Box<dyn Decoder>> {
        &mut self.0
    }
}

impl Default for DecoderRegistry {
    fn default() -> Self {
        let mut registry = HashMap::with_capacity(6);
        registry.insert(Compression::None, Box::new(UncompressedDecoder) as _);
        registry.insert(Compression::Deflate, Box::new(DeflateDecoder) as _);
        registry.insert(Compression::OldDeflate, Box::new(DeflateDecoder) as _);
        #[cfg(feature = "lerc")]
        registry.insert(Compression::LERC, Box::new(LercDecoder) as _);
        #[cfg(feature = "lzma")]
        registry.insert(Compression::LZMA, Box::new(LZMADecoder) as _);
        registry.insert(Compression::LZW, Box::new(LZWDecoder) as _);
        registry.insert(Compression::ModernJPEG, Box::new(JPEGDecoder) as _);
        #[cfg(feature = "jpeg2k")]
        registry.insert(Compression::JPEG2k, Box::new(JPEG2kDecoder) as _);
        #[cfg(feature = "webp")]
        registry.insert(Compression::WebP, Box::new(WebPDecoder) as _);
        registry.insert(Compression::ZSTD, Box::new(ZstdDecoder) as _);
        Self(registry)
    }
}

/// A trait to decode a TIFF tile.
pub trait Decoder: Debug + Send + Sync {
    /// Decode a TIFF tile.
    fn decode_tile(
        &self,
        buffer: Bytes,
        photometric_interpretation: PhotometricInterpretation,
        jpeg_tables: Option<&[u8]>,
        samples_per_pixel: u16,
        bits_per_sample: u16,
        lerc_parameters: Option<&[u32]>,
    ) -> AsyncTiffResult<Vec<u8>>;
}

/// A decoder for the Deflate compression method.
#[derive(Debug, Clone)]
pub struct DeflateDecoder;

impl Decoder for DeflateDecoder {
    fn decode_tile(
        &self,
        buffer: Bytes,
        _photometric_interpretation: PhotometricInterpretation,
        _jpeg_tables: Option<&[u8]>,
        _samples_per_pixel: u16,
        _bits_per_sample: u16,
        _lerc_parameters: Option<&[u32]>,
    ) -> AsyncTiffResult<Vec<u8>> {
        let mut decoder = ZlibDecoder::new(Cursor::new(buffer));
        let mut buf = Vec::new();
        decoder.read_to_end(&mut buf)?;
        Ok(buf)
    }
}

/// A decoder for the JPEG compression method.
#[derive(Debug, Clone)]
pub struct JPEGDecoder;

impl Decoder for JPEGDecoder {
    fn decode_tile(
        &self,
        buffer: Bytes,
        photometric_interpretation: PhotometricInterpretation,
        jpeg_tables: Option<&[u8]>,
        _samples_per_pixel: u16,
        _bits_per_sample: u16,
        _lerc_parameters: Option<&[u32]>,
    ) -> AsyncTiffResult<Vec<u8>> {
        decode_modern_jpeg(buffer, photometric_interpretation, jpeg_tables)
    }
}

/// A decoder for the LERC compression method.
#[cfg(feature = "lerc")]
#[derive(Debug, Clone)]
pub struct LercDecoder;

/// Helper to decode and convert to bytes
#[cfg(feature = "lerc")]
fn decode_lerc<T: lerc::LercDataType + bytemuck::Pod>(
    buffer: &[u8],
    info: &lerc::BlobInfo,
) -> AsyncTiffResult<Vec<u8>> {
    let (data, _mask) = lerc::decode::<T>(
        buffer,
        info.width as usize,
        info.height as usize,
        info.depth as usize,
        info.bands as usize,
        info.masks as usize,
    )
    .map_err(|e| AsyncTiffError::General(format!("LERC decode failed: {e}")))?;

    // TODO: in the future we could avoid this copy by allowing the return type of the decoder to
    // be a typed array, not just Vec<u8>
    Ok(bytemuck::cast_slice(&data).to_vec())
}

#[cfg(feature = "lerc")]
impl Decoder for LercDecoder {
    fn decode_tile(
        &self,
        buffer: Bytes,
        _photometric_interpretation: PhotometricInterpretation,
        _jpeg_tables: Option<&[u8]>,
        _samples_per_pixel: u16,
        _bits_per_sample: u16,
        lerc_parameters: Option<&[u32]>,
    ) -> AsyncTiffResult<Vec<u8>> {
        // LercParameters[1] is the inner compression type:
        //   0 = none, 1 = deflate, 2 = zstd
        // Decompress the outer wrapper before passing to the LERC decoder.
        let lerc_blob: Vec<u8> = match lerc_parameters.and_then(|p| p.get(1).copied()) {
            Some(1) => {
                let mut decoder = ZlibDecoder::new(Cursor::new(buffer));
                let mut buf = Vec::new();
                decoder.read_to_end(&mut buf)?;
                buf
            }
            Some(2) => {
                let mut decoder = zstd::Decoder::new(Cursor::new(buffer))?;
                let mut buf = Vec::new();
                decoder.read_to_end(&mut buf)?;
                buf
            }
            _ => buffer.to_vec(),
        };

        let info = lerc::get_blob_info(&lerc_blob)
            .map_err(|e| AsyncTiffError::General(format!("LERC get_blob_info failed: {e}")))?;

        // LERC data_type mapping (from LERC C API):
        // 0=i8, 1=u8, 2=i16, 3=u16, 4=i32, 5=u32, 6=f32, 7=f64
        match info.data_type {
            0 => decode_lerc::<i8>(&lerc_blob, &info),
            1 => decode_lerc::<u8>(&lerc_blob, &info),
            2 => decode_lerc::<i16>(&lerc_blob, &info),
            3 => decode_lerc::<u16>(&lerc_blob, &info),
            4 => decode_lerc::<i32>(&lerc_blob, &info),
            5 => decode_lerc::<u32>(&lerc_blob, &info),
            6 => decode_lerc::<f32>(&lerc_blob, &info),
            7 => decode_lerc::<f64>(&lerc_blob, &info),
            _ => Err(AsyncTiffError::General(format!(
                "Unsupported LERC data type: {}",
                info.data_type
            ))),
        }
    }
}

/// A decoder for the LZMA compression method.
#[derive(Debug, Clone)]
#[cfg(feature = "lzma")]
pub struct LZMADecoder;

#[cfg(feature = "lzma")]
impl Decoder for LZMADecoder {
    fn decode_tile(
        &self,
        buffer: Bytes,
        _photometric_interpretation: PhotometricInterpretation,
        _jpeg_tables: Option<&[u8]>,
        _samples_per_pixel: u16,
        _bits_per_sample: u16,
        _lerc_parameters: Option<&[u32]>,
    ) -> AsyncTiffResult<Vec<u8>> {
        use bytes::Buf;
        use lzma_rust2::XzReader;

        let mut reader = XzReader::new(buffer.reader(), false);
        let mut out = Vec::new();
        reader.read_to_end(&mut out)?;
        Ok(out)
    }
}

/// A decoder for the LZW compression method.
#[derive(Debug, Clone)]
pub struct LZWDecoder;

impl Decoder for LZWDecoder {
    fn decode_tile(
        &self,
        buffer: Bytes,
        _photometric_interpretation: PhotometricInterpretation,
        _jpeg_tables: Option<&[u8]>,
        _samples_per_pixel: u16,
        _bits_per_sample: u16,
        _lerc_parameters: Option<&[u32]>,
    ) -> AsyncTiffResult<Vec<u8>> {
        // https://github.com/image-rs/image-tiff/blob/90ae5b8e54356a35e266fb24e969aafbcb26e990/src/decoder/stream.rs#L147
        let mut decoder = weezl::decode::Decoder::with_tiff_size_switch(weezl::BitOrder::Msb, 8);
        let decoded = decoder.decode(&buffer).expect("failed to decode LZW data");
        Ok(decoded)
    }
}

/// A decoder for the JPEG2000 compression method.
#[cfg(feature = "jpeg2k")]
#[derive(Debug, Clone)]
pub struct JPEG2kDecoder;

#[cfg(feature = "jpeg2k")]
impl Decoder for JPEG2kDecoder {
    fn decode_tile(
        &self,
        buffer: Bytes,
        _photometric_interpretation: PhotometricInterpretation,
        _jpeg_tables: Option<&[u8]>,
        _samples_per_pixel: u16,
        _bits_per_sample: u16,
        _lerc_parameters: Option<&[u32]>,
    ) -> AsyncTiffResult<Vec<u8>> {
        let decoder = jpeg2k::DecodeParameters::new();

        let image = jpeg2k::Image::from_bytes_with(&buffer, decoder)?;

        let id = image.get_pixels(None)?;
        match id.data {
            jpeg2k::ImagePixelData::L8(items)
            | jpeg2k::ImagePixelData::La8(items)
            | jpeg2k::ImagePixelData::Rgb8(items)
            | jpeg2k::ImagePixelData::Rgba8(items) => Ok(items),
            jpeg2k::ImagePixelData::L16(items)
            | jpeg2k::ImagePixelData::La16(items)
            | jpeg2k::ImagePixelData::Rgb16(items)
            | jpeg2k::ImagePixelData::Rgba16(items) => Ok(bytemuck::cast_vec(items)),
        }
    }
}

/// A decoder for the WebP compression method.
#[cfg(feature = "webp")]
#[derive(Debug, Clone)]
pub struct WebPDecoder;

#[cfg(feature = "webp")]
impl Decoder for WebPDecoder {
    fn decode_tile(
        &self,
        buffer: Bytes,
        _photometric_interpretation: PhotometricInterpretation,
        _jpeg_tables: Option<&[u8]>,
        samples_per_pixel: u16,
        bits_per_sample: u16,
        _lerc_parameters: Option<&[u32]>,
    ) -> AsyncTiffResult<Vec<u8>> {
        let decoded = webp::Decoder::new(&buffer)
            .decode()
            .ok_or(AsyncTiffError::General("WebP decoding failed".to_string()))?;

        let data = decoded.to_vec();

        // WebP lossy compression may discard fully-opaque alpha channels.
        // If the TIFF expects 4 samples but WebP decoded to 3, expand RGB to RGBA.
        // Only do this for 8-bit data since WebP only supports 8-bit.
        if samples_per_pixel == 4 && bits_per_sample == 8 && !decoded.is_alpha() {
            let mut rgba = Vec::with_capacity(data.len() / 3 * 4);
            for chunk in data.chunks_exact(3) {
                rgba.extend_from_slice(chunk);
                rgba.push(255); // opaque alpha
            }
            Ok(rgba)
        } else {
            Ok(data)
        }
    }
}

/// A decoder for uncompressed data.
#[derive(Debug, Clone)]
pub struct UncompressedDecoder;

impl Decoder for UncompressedDecoder {
    fn decode_tile(
        &self,
        buffer: Bytes,
        _photometric_interpretation: PhotometricInterpretation,
        _jpeg_tables: Option<&[u8]>,
        _samples_per_pixel: u16,
        _bits_per_sample: u16,
        _lerc_parameters: Option<&[u32]>,
    ) -> AsyncTiffResult<Vec<u8>> {
        Ok(buffer.to_vec())
    }
}

/// A decoder for the Zstd compression method.
#[derive(Debug, Clone)]
pub struct ZstdDecoder;

impl Decoder for ZstdDecoder {
    fn decode_tile(
        &self,
        buffer: Bytes,
        _photometric_interpretation: PhotometricInterpretation,
        _jpeg_tables: Option<&[u8]>,
        _samples_per_pixel: u16,
        _bits_per_sample: u16,
        _lerc_parameters: Option<&[u32]>,
    ) -> AsyncTiffResult<Vec<u8>> {
        let mut decoder = zstd::Decoder::new(Cursor::new(buffer))?;
        let mut buf = Vec::new();
        decoder.read_to_end(&mut buf)?;
        Ok(buf)
    }
}

// https://github.com/image-rs/image-tiff/blob/3bfb43e83e31b0da476832067ada68a82b378b7b/src/decoder/image.rs#L389-L450
fn decode_modern_jpeg(
    buf: Bytes,
    photometric_interpretation: PhotometricInterpretation,
    jpeg_tables: Option<&[u8]>,
) -> AsyncTiffResult<Vec<u8>> {
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
    let reader = Cursor::new(buf);

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
    Ok(data)
}
