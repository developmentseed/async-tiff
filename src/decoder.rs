//! Decoders for different TIFF compression methods.

use std::collections::HashMap;
use std::fmt::Debug;
use std::io::{Cursor, Read};
use std::panic;

use bytes::Bytes;
use flate2::bufread::ZlibDecoder;
use mozjpeg::{ColorSpace as MozColorSpace, Decompress};

use crate::error::{AsyncTiffError, AsyncTiffResult, TiffError, TiffUnsupportedError};
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
        reference_black_white: Option<&[f64; 6]>,
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
        _reference_black_white: Option<&[f64; 6]>,
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
        reference_black_white: Option<&[f64; 6]>,
    ) -> AsyncTiffResult<Vec<u8>> {
        decode_modern_jpeg(buffer, photometric_interpretation, jpeg_tables, reference_black_white)
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
        _reference_black_white: Option<&[f64; 6]>,
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
        _reference_black_white: Option<&[f64; 6]>,
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
        _reference_black_white: Option<&[f64; 6]>,
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
        _reference_black_white: Option<&[f64; 6]>,
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
        _reference_black_white: Option<&[f64; 6]>,
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
        _reference_black_white: Option<&[f64; 6]>,
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
        _reference_black_white: Option<&[f64; 6]>,
    ) -> AsyncTiffResult<Vec<u8>> {
        let mut decoder = zstd::Decoder::new(Cursor::new(buffer))?;
        let mut buf = Vec::new();
        decoder.read_to_end(&mut buf)?;
        Ok(buf)
    }
}

fn decode_modern_jpeg(
    buf: Bytes,
    photometric_interpretation: PhotometricInterpretation,
    jpeg_tables: Option<&[u8]>,
    reference_black_white: Option<&[f64; 6]>,
) -> AsyncTiffResult<Vec<u8>> {
    // JPEG compression in TIFF allows saving quantization and/or huffman tables in one central
    // location. These `jpeg_tables` are simply prepended to the remaining jpeg image data.
    // `jpeg_tables` starts with SOI (0xFFD8) and ends with EOI (0xFFD9); strip them off before
    // concatenating with the tile data (which starts with its own SOI).
    let jpeg_data: Vec<u8> = match jpeg_tables {
        Some(tables) => {
            // Strip SOI from tile data (first 2 bytes) and EOI from tables (last 2 bytes),
            // then prepend the stripped tables to the tile data.
            let stripped_tables = &tables[..tables.len() - 2];
            let stripped_tile = &buf[2..];
            let mut data = Vec::with_capacity(stripped_tables.len() + stripped_tile.len());
            data.extend_from_slice(stripped_tables);
            data.extend_from_slice(stripped_tile);
            data
        }
        None => buf.to_vec(),
    };

    // mozjpeg (libjpeg-turbo) uses panic-based error handling internally.
    // All mozjpeg calls must be wrapped in catch_unwind per the library's requirements.
    // See: https://docs.rs/mozjpeg/latest/mozjpeg/
    let result = panic::catch_unwind(|| -> std::io::Result<Vec<u8>> {
        // Use mozjpeg (libjpeg-turbo) for decoding to match libtiff/GDAL/rasterio output exactly.
        // For YCbCr, decode to raw YCbCr (no internal color conversion) then apply the
        // TIFF-correct color conversion using the ReferenceBlackWhite tag.
        if photometric_interpretation == PhotometricInterpretation::YCbCr {
            // Decode to upsampled interleaved YCbCr (no internal color conversion by libjpeg),
            // then apply the TIFF-correct color conversion using the ReferenceBlackWhite tag.
            let mut decompress = Decompress::new_mem(&jpeg_data)?
                .to_colorspace(MozColorSpace::JCS_YCbCr)?;
            let ycbcr = decompress.read_scanlines::<u8>()?;
            return Ok(ycbcr_to_rgb(&ycbcr, reference_black_white));
        }

        let moz_cs = match photometric_interpretation {
            PhotometricInterpretation::RGB => MozColorSpace::JCS_RGB,
            PhotometricInterpretation::WhiteIsZero
            | PhotometricInterpretation::BlackIsZero
            | PhotometricInterpretation::TransparencyMask => MozColorSpace::JCS_GRAYSCALE,
            PhotometricInterpretation::CMYK => MozColorSpace::JCS_CMYK,
            _ => {
                // Unsupported colorspaces are handled outside catch_unwind via the outer match.
                // Return an empty vec as sentinel; the outer code handles the error.
                return Ok(vec![]);
            }
        };

        let mut decompress = Decompress::new_mem(&jpeg_data)?.to_colorspace(moz_cs)?;
        decompress.read_scanlines::<u8>()
    });

    // Handle unsupported photometric interpretation (avoids passing non-UnwindSafe types into closure)
    if !matches!(
        photometric_interpretation,
        PhotometricInterpretation::RGB
            | PhotometricInterpretation::YCbCr
            | PhotometricInterpretation::WhiteIsZero
            | PhotometricInterpretation::BlackIsZero
            | PhotometricInterpretation::TransparencyMask
            | PhotometricInterpretation::CMYK
    ) {
        return Err(TiffError::UnsupportedError(
            TiffUnsupportedError::UnsupportedInterpretation(photometric_interpretation),
        )
        .into());
    }

    result
        .map_err(|_| AsyncTiffError::General("JPEG decode panicked".to_string()))?
        .map_err(|e| AsyncTiffError::General(format!("JPEG decode error: {e}")))
}

/// Convert upsampled YCbCr pixels to RGB replicating libtiff's exact fixed-point arithmetic.
///
/// Replicates `TIFFYCbCrToRGBInit` + `TIFFYCbCrtoRGB` from libtiff/tif_color.c, including
/// the `Code2V` ReferenceBlackWhite scaling and the 16-bit fixed-point lookup tables.
fn ycbcr_to_rgb(ycbcr: &[u8], reference_black_white: Option<&[f64; 6]>) -> Vec<u8> {
    // Default TIFF ReferenceBlackWhite for YCbCr
    let rbw =
        reference_black_white.copied().unwrap_or([0.0, 255.0, 128.0, 255.0, 128.0, 255.0]);
    let [rb0, rw0, rb2, rw2, rb4, rw4] = rbw;

    // libtiff uses SHIFT=16, ONE_HALF=1<<15, FIX(x) = (x*(1<<16)+0.5) as i32
    const SHIFT: u32 = 16;
    const ONE_HALF: i64 = 1 << 15;

    // CCIR 601 luma coefficients (ITU-R BT.601)
    const LUMA_RED: f64 = 0.299;
    const LUMA_GREEN: f64 = 0.587;
    const LUMA_BLUE: f64 = 0.114;

    let fix = |x: f64| -> i64 { (x * (1i64 << SHIFT) as f64 + 0.5) as i64 };

    // libtiff D1..D4 fixed-point matrix coefficients
    let d1 = fix(2.0 - 2.0 * LUMA_RED); // Cr -> R
    let d2 = fix(-(LUMA_RED * (2.0 - 2.0 * LUMA_RED) / LUMA_GREEN)); // Cr -> G
    let d3 = fix(2.0 - 2.0 * LUMA_BLUE); // Cb -> B
    let d4 = fix(-(LUMA_BLUE * (2.0 - 2.0 * LUMA_BLUE) / LUMA_GREEN)); // Cb -> G

    // Code2V: maps raw code to scaled value using ReferenceBlackWhite
    let code2v = |c: i32, rb: f64, rw: f64, cr: f64| -> i32 {
        let denom = if (rw - rb).abs() > f64::EPSILON { rw - rb } else { 1.0 };
        // CLAMPw to [-128*32, 128*32]
        ((c as f64 - rb) * cr / denom).clamp(-128.0 * 32.0, 128.0 * 32.0) as i32
    };

    // Build lookup tables indexed by raw 0..=255 pixel values.
    // libtiff loop: for (i=0, x=-128; i<256; i++, x++)
    let mut y_tab = [0i32; 256];
    let mut cr_r_tab = [0i32; 256];
    let mut cb_b_tab = [0i32; 256];
    let mut cr_g_tab = [0i64; 256]; // stored unshifted
    let mut cb_g_tab = [0i64; 256]; // stored unshifted + ONE_HALF

    for i in 0usize..256 {
        let x = i as i32 - 128;
        y_tab[i] = code2v(x + 128, rb0, rw0, 255.0);

        let cr = code2v(x, rb4 - 128.0, rw4 - 128.0, 127.0) as i64;
        cr_r_tab[i] = ((d1 * cr + ONE_HALF) >> SHIFT) as i32;
        cr_g_tab[i] = d2 * cr;

        let cb = code2v(x, rb2 - 128.0, rw2 - 128.0, 127.0) as i64;
        cb_b_tab[i] = ((d3 * cb + ONE_HALF) >> SHIFT) as i32;
        cb_g_tab[i] = d4 * cb + ONE_HALF;
    }

    let n_pixels = ycbcr.len() / 3;
    let mut rgb = vec![0u8; n_pixels * 3];

    for i in 0..n_pixels {
        let y = ycbcr[i * 3] as usize;
        let cb = ycbcr[i * 3 + 1] as usize;
        let cr = ycbcr[i * 3 + 2] as usize;

        let yv = y_tab[y];
        let r = yv + cr_r_tab[cr];
        let g = yv + ((cb_g_tab[cb] + cr_g_tab[cr]) >> SHIFT) as i32;
        let b = yv + cb_b_tab[cb];

        rgb[i * 3] = r.clamp(0, 255) as u8;
        rgb[i * 3 + 1] = g.clamp(0, 255) as u8;
        rgb[i * 3 + 2] = b.clamp(0, 255) as u8;
    }

    rgb
}
