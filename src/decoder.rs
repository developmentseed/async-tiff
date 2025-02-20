use std::io::{Cursor, Read};

use bytes::Bytes;
use flate2::bufread::ZlibDecoder;
use tiff::tags::{CompressionMethod, PhotometricInterpretation};
use tiff::{TiffError, TiffUnsupportedError};

use crate::error::Result;

// https://github.com/image-rs/image-tiff/blob/3bfb43e83e31b0da476832067ada68a82b378b7b/src/decoder/image.rs#L370
pub(crate) fn decode_tile(
    buf: Bytes,
    photometric_interpretation: PhotometricInterpretation,
    compression_method: CompressionMethod,
    // compressed_length: u64,
    jpeg_tables: Option<&Vec<u8>>,
) -> Result<Vec<u8>> {
    match compression_method {
        CompressionMethod::None => Ok(buf.to_vec()),
        CompressionMethod::LZW => decode_lzw(buf),
        CompressionMethod::Deflate | CompressionMethod::OldDeflate => decode_deflate(buf),
        CompressionMethod::ModernJPEG => {
            decode_modern_jpeg(buf, photometric_interpretation, jpeg_tables)
        }
        method => Err(TiffError::UnsupportedError(
            TiffUnsupportedError::UnsupportedCompressionMethod(method),
        )
        .into()),
    }
}

fn decode_lzw(buf: Bytes) -> Result<Vec<u8>> {
    // https://github.com/image-rs/image-tiff/blob/90ae5b8e54356a35e266fb24e969aafbcb26e990/src/decoder/stream.rs#L147
    let mut decoder = weezl::decode::Decoder::with_tiff_size_switch(weezl::BitOrder::Msb, 8);
    let decoded = decoder.decode(&buf).expect("failed to decode LZW data");
    Ok(decoded)
}

fn decode_deflate(buf: Bytes) -> Result<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(Cursor::new(buf));
    let mut buf = Vec::new();
    decoder.read_to_end(&mut buf)?;
    Ok(buf)
}

// https://github.com/image-rs/image-tiff/blob/3bfb43e83e31b0da476832067ada68a82b378b7b/src/decoder/image.rs#L389-L450
fn decode_modern_jpeg(
    buf: Bytes,
    photometric_interpretation: PhotometricInterpretation,
    jpeg_tables: Option<&Vec<u8>>,
) -> Result<Vec<u8>> {
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
        PhotometricInterpretation::WhiteIsZero => {
            decoder.set_color_transform(jpeg::ColorTransform::None)
        }
        PhotometricInterpretation::BlackIsZero => {
            decoder.set_color_transform(jpeg::ColorTransform::None)
        }
        PhotometricInterpretation::TransparencyMask => {
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

trait Decode {
    // TODO: should this return an ndarray?
    fn decompress(&self, tile: Vec<u8>) -> Vec<u8>;
}

pub(crate) struct ModernJPEGDecoder {
    tile: Vec<u8>,
    jpeg_tables: Vec<u8>,
}

impl Decode for ModernJPEGDecoder {
    fn decompress(&self, tile: Vec<u8>) -> Vec<u8> {
        todo!()
    }
}

pub(crate) struct LZWDecompressor {}

impl Decode for LZWDecompressor {
    fn decompress(&self, tile: Vec<u8>) -> Vec<u8> {
        todo!()
    }
}

pub(crate) struct WebPDecompressor {}

impl Decode for WebPDecompressor {
    fn decompress(&self, tile: Vec<u8>) -> Vec<u8> {
        todo!()
    }
}

pub(crate) struct DeflateDecompressor {}

impl Decode for DeflateDecompressor {
    fn decompress(&self, tile: Vec<u8>) -> Vec<u8> {
        todo!()
    }
}

pub(crate) struct PackbitsDecompressor {}

impl Decode for PackbitsDecompressor {
    fn decompress(&self, tile: Vec<u8>) -> Vec<u8> {
        todo!()
    }
}
