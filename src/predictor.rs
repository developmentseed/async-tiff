//! Predictors for no predictor, horizontal and floating-point
use std::fmt::Debug;

use bytes::{Bytes, BytesMut};

use crate::error::AsyncTiffError;
use crate::tiff::tags::PlanarConfiguration;
use crate::ImageFileDirectory;
use crate::{error::AsyncTiffResult, reader::Endianness};

/// All info that may be used by a predictor
///
/// Most of this is used by the floating point predictor
/// since that intermixes padding into the decompressed output
///
/// Also provides convenience functions
///
#[derive(Debug, Clone, Copy)]
pub(crate) struct PredictorInfo {
    /// endianness
    endianness: Endianness,
    /// width of the image in pixels
    image_width: u32,
    /// height of the image in pixels
    image_height: u32,
    /// chunk width in pixels
    ///
    /// If this is a stripped tiff, `chunk_width=image_width`
    chunk_width: u32,
    /// chunk height in pixels
    chunk_height: u32,
    /// bits per sample
    ///
    /// We only support a single bits_per_sample across all samples
    bits_per_sample: u16,
    /// number of samples per pixel
    samples_per_pixel: u16,

    planar_configuration: PlanarConfiguration,
}

impl PredictorInfo {
    pub(crate) fn endianness(&self) -> Endianness {
        self.endianness
    }

    pub(crate) fn bits_per_sample(&self) -> u16 {
        self.bits_per_sample
    }

    pub(crate) fn from_ifd(ifd: &ImageFileDirectory) -> Self {
        if !ifd.bits_per_sample.windows(2).all(|w| w[0] == w[1]) {
            panic!("bits_per_sample should be the same for all channels");
        }

        let chunk_width = if let Some(tile_width) = ifd.tile_width {
            tile_width
        } else {
            ifd.image_width
        };
        let chunk_height = if let Some(tile_height) = ifd.tile_height {
            tile_height
        } else {
            ifd.rows_per_strip
                .expect("no tile height and no rows_per_strip")
        };

        PredictorInfo {
            endianness: ifd.endianness,
            image_width: ifd.image_width,
            image_height: ifd.image_height,
            chunk_width,
            chunk_height,
            planar_configuration: ifd.planar_configuration,
            bits_per_sample: ifd.bits_per_sample[0],
            samples_per_pixel: ifd.samples_per_pixel,
        }
    }

    /// chunk width in pixels, taking padding into account
    ///
    /// strips are considered image-width chunks
    fn chunk_width_pixels(&self, x: u32) -> AsyncTiffResult<u32> {
        let chunks_across = self.chunks_across();
        if x >= chunks_across {
            Err(AsyncTiffError::TileIndexError(x, chunks_across))
        } else if x == chunks_across - 1 {
            // last chunk
            Ok(self.image_width - self.chunk_width * x)
        } else {
            Ok(self.chunk_width)
        }
    }

    /// chunk height in pixels, taking padding into account
    ///
    /// strips are considered image-width chunks
    fn chunk_height_pixels(&self, y: u32) -> AsyncTiffResult<u32> {
        let chunks_down = self.chunks_down();
        if y >= chunks_down {
            Err(AsyncTiffError::TileIndexError(y, chunks_down))
        } else if y == chunks_down - 1 {
            // last chunk
            Ok(self.image_height - self.chunk_height * y)
        } else {
            Ok(self.chunk_height)
        }
    }

    /// get the output row stride in bytes, taking padding into account
    fn output_row_stride(&self, x: u32) -> AsyncTiffResult<usize> {
        Ok((self.chunk_width_pixels(x)? as usize).saturating_mul(self.bits_per_pixel()) / 8)
    }

    /// the number of rows the output has, taking padding and PlanarConfiguration into account.
    fn output_rows(&self, y: u32) -> AsyncTiffResult<usize> {
        match self.planar_configuration {
            PlanarConfiguration::Chunky => Ok(self.chunk_height_pixels(y)? as usize),
            PlanarConfiguration::Planar => {
                Ok((self.chunk_height_pixels(y)? as usize)
                    .saturating_mul(self.samples_per_pixel as _))
            }
        }
    }

    fn bits_per_pixel(&self) -> usize {
        match self.planar_configuration {
            PlanarConfiguration::Chunky => {
                self.bits_per_sample as usize * self.samples_per_pixel as usize
            }
            PlanarConfiguration::Planar => self.bits_per_sample as usize,
        }
    }

    /// The number of chunks in the horizontal (x) direction
    fn chunks_across(&self) -> u32 {
        self.image_width.div_ceil(self.chunk_width)
    }

    /// The number of chunks in the vertical (y) direction
    fn chunks_down(&self) -> u32 {
        self.image_height.div_ceil(self.chunk_height)
    }
}

/// reverse horizontal predictor
///
/// fixes byte order before reversing differencing
pub(crate) fn unpredict_hdiff(
    buffer: Bytes,
    predictor_info: &PredictorInfo,
    tile_x: u32,
) -> AsyncTiffResult<Bytes> {
    let output_row_stride = predictor_info.output_row_stride(tile_x)?;
    let samples = predictor_info.samples_per_pixel as usize;
    let bit_depth = predictor_info.bits_per_sample;

    let buffer = fix_endianness(buffer, predictor_info.endianness, bit_depth);
    let mut res = BytesMut::from(buffer);
    for buf in res.chunks_mut(output_row_stride) {
        rev_hpredict_nsamp(buf, bit_depth, samples);
    }
    Ok(res.into())
}

/// Reverse predictor convenience function for horizontal differencing
///
/// From image-tiff
///
/// This should be used _after_ endianness fixing
pub fn rev_hpredict_nsamp(buf: &mut [u8], bit_depth: u16, samples: usize) {
    match bit_depth {
        0..=8 => {
            for i in samples..buf.len() {
                buf[i] = buf[i].wrapping_add(buf[i - samples]);
            }
        }
        9..=16 => {
            for i in (samples * 2..buf.len()).step_by(2) {
                let v = u16::from_ne_bytes(buf[i..][..2].try_into().unwrap());
                let p = u16::from_ne_bytes(buf[i - 2 * samples..][..2].try_into().unwrap());
                buf[i..][..2].copy_from_slice(&(v.wrapping_add(p)).to_ne_bytes());
            }
        }
        17..=32 => {
            for i in (samples * 4..buf.len()).step_by(4) {
                let v = u32::from_ne_bytes(buf[i..][..4].try_into().unwrap());
                let p = u32::from_ne_bytes(buf[i - 4 * samples..][..4].try_into().unwrap());
                buf[i..][..4].copy_from_slice(&(v.wrapping_add(p)).to_ne_bytes());
            }
        }
        33..=64 => {
            for i in (samples * 8..buf.len()).step_by(8) {
                let v = u64::from_ne_bytes(buf[i..][..8].try_into().unwrap());
                let p = u64::from_ne_bytes(buf[i - 8 * samples..][..8].try_into().unwrap());
                buf[i..][..8].copy_from_slice(&(v.wrapping_add(p)).to_ne_bytes());
            }
        }
        _ => {
            unreachable!("Caller should have validated arguments. Please file a bug.")
        }
    }
}

/// Fix endianness. If `byte_order` matches the host, then conversion is a no-op.
///
/// from image-tiff
pub fn fix_endianness(buffer: Bytes, byte_order: Endianness, bit_depth: u16) -> Bytes {
    #[cfg(target_endian = "little")]
    if let Endianness::LittleEndian = byte_order {
        return buffer;
    }
    #[cfg(target_endian = "big")]
    if let Endianness::BigEndian = byte_order {
        return buffer;
    }

    match byte_order {
        Endianness::LittleEndian => {
            let mut buf = BytesMut::from(buffer);
            match bit_depth {
                0..=8 => {}
                9..=16 => buf.chunks_exact_mut(2).for_each(|v| {
                    v.copy_from_slice(&u16::from_le_bytes((*v).try_into().unwrap()).to_ne_bytes())
                }),
                17..=32 => buf.chunks_exact_mut(4).for_each(|v| {
                    v.copy_from_slice(&u32::from_le_bytes((*v).try_into().unwrap()).to_ne_bytes())
                }),
                _ => buf.chunks_exact_mut(8).for_each(|v| {
                    v.copy_from_slice(&u64::from_le_bytes((*v).try_into().unwrap()).to_ne_bytes())
                }),
            }
            buf.freeze()
        }
        Endianness::BigEndian => {
            let mut buf = BytesMut::from(buffer);
            match bit_depth {
                0..=8 => {}
                9..=16 => buf.chunks_exact_mut(2).for_each(|v| {
                    v.copy_from_slice(&u16::from_be_bytes((*v).try_into().unwrap()).to_ne_bytes())
                }),
                17..=32 => buf.chunks_exact_mut(4).for_each(|v| {
                    v.copy_from_slice(&u32::from_be_bytes((*v).try_into().unwrap()).to_ne_bytes())
                }),
                _ => buf.chunks_exact_mut(8).for_each(|v| {
                    v.copy_from_slice(&u64::from_be_bytes((*v).try_into().unwrap()).to_ne_bytes())
                }),
            };
            buf.freeze()
        }
    }
}

/// Reverse a floating-point prediction
///
/// According to [the spec](http://chriscox.org/TIFFTN3d1.pdf), no external
/// byte-ordering should be done.
///
/// If the tile has horizontal padding, it will shorten the output.
pub(crate) fn unpredict_float(
    buffer: Bytes,
    predictor_info: &PredictorInfo,
    tile_x: u32,
    tile_y: u32,
) -> AsyncTiffResult<Bytes> {
    let output_row_stride = predictor_info.output_row_stride(tile_x)?;
    let mut res: BytesMut =
        BytesMut::zeroed(output_row_stride * predictor_info.output_rows(tile_y)?);
    let bit_depth = predictor_info.bits_per_sample;
    if predictor_info.chunk_width_pixels(tile_x)? == predictor_info.chunk_width {
        // no special padding handling
        let mut input = BytesMut::from(buffer);
        for (in_buf, out_buf) in input
            .chunks_mut(output_row_stride)
            .zip(res.chunks_mut(output_row_stride))
        {
            match bit_depth {
                16 => rev_predict_f16(in_buf, out_buf, predictor_info.samples_per_pixel as _),
                32 => rev_predict_f32(in_buf, out_buf, predictor_info.samples_per_pixel as _),
                64 => rev_predict_f64(in_buf, out_buf, predictor_info.samples_per_pixel as _),
                _ => {
                    return Err(AsyncTiffError::General(format!(
                        "thou shalt not predict f{bit_depth:?}"
                    )))
                }
            }
        }
    } else {
        // specially handle padding bytes
        // create a buffer for the full width
        let mut input = BytesMut::from(buffer);

        let input_row_stride =
            predictor_info.chunk_width as usize * predictor_info.bits_per_sample as usize / 8;
        for (in_buf, out_buf) in input
            .chunks_mut(input_row_stride)
            .zip(res.chunks_mut(output_row_stride))
        {
            let mut out_row = BytesMut::zeroed(input_row_stride);
            match bit_depth {
                16 => rev_predict_f16(in_buf, &mut out_row, predictor_info.samples_per_pixel as _),
                32 => rev_predict_f32(in_buf, &mut out_row, predictor_info.samples_per_pixel as _),
                64 => rev_predict_f64(in_buf, &mut out_row, predictor_info.samples_per_pixel as _),
                _ => {
                    return Err(AsyncTiffError::General(format!(
                        "thou shalt not predict f{bit_depth:?}"
                    )))
                }
            }
            // remove the padding bytes
            out_buf.copy_from_slice(&out_row[..output_row_stride]);
        }
    }
    Ok(res.into())
}

/// Reverse floating point prediction
///
/// floating point prediction first shuffles the bytes and then uses horizontal
/// differencing
/// also performs byte-order conversion if needed.
pub fn rev_predict_f16(input: &mut [u8], output: &mut [u8], samples: usize) {
    // reverse horizontal differencing
    for i in samples..input.len() {
        input[i] = input[i].wrapping_add(input[i - samples]);
    }
    // reverse byte shuffle and fix endianness
    for (i, chunk) in output.chunks_mut(2).enumerate() {
        chunk.copy_from_slice(&u16::to_ne_bytes(
            // convert to native-endian
            // floating predictor is be-like
            u16::from_be_bytes([input[i], input[input.len() / 2 + i]]),
        ));
    }
}

/// Reverse floating point prediction
///
/// floating point prediction first shuffles the bytes and then uses horizontal
/// differencing
/// also performs byte-order conversion if needed.
pub fn rev_predict_f32(input: &mut [u8], output: &mut [u8], samples: usize) {
    // reverse horizontal differencing
    for i in samples..input.len() {
        input[i] = input[i].wrapping_add(input[i - samples]);
    }
    // reverse byte shuffle and fix endianness
    for (i, chunk) in output.chunks_mut(4).enumerate() {
        chunk.copy_from_slice(
            // convert to native-endian
            &u32::to_ne_bytes(
                // floating predictor is be-like
                u32::from_be_bytes([
                    input[i],
                    input[input.len() / 4 + i],
                    input[input.len() / 4 * 2 + i],
                    input[input.len() / 4 * 3 + i],
                ]),
            ),
        );
    }
}

/// Reverse floating point prediction
///
/// floating point prediction first shuffles the bytes and then uses horizontal
/// differencing
/// Also fixes byte order if needed (tiff's->native)
pub fn rev_predict_f64(input: &mut [u8], output: &mut [u8], samples: usize) {
    for i in samples..input.len() {
        input[i] = input[i].wrapping_add(input[i - samples]);
    }

    for (i, chunk) in output.chunks_mut(8).enumerate() {
        chunk.copy_from_slice(
            // convert to native-endian
            &u64::to_ne_bytes(
                // floating predictor is be-like
                u64::from_be_bytes([
                    input[i],
                    input[input.len() / 8 + i],
                    input[input.len() / 8 * 2 + i],
                    input[input.len() / 8 * 3 + i],
                    input[input.len() / 8 * 4 + i],
                    input[input.len() / 8 * 5 + i],
                    input[input.len() / 8 * 6 + i],
                    input[input.len() / 8 * 7 + i],
                ]),
            ),
        );
    }
}

#[cfg(test)]
mod test {
    use std::vec;

    use bytes::Bytes;

    use crate::{
        predictor::{unpredict_float, unpredict_hdiff},
        reader::Endianness,
    };

    use super::*;

    const PRED_INFO: PredictorInfo = PredictorInfo {
        endianness: Endianness::LittleEndian,
        image_width: 7,
        image_height: 7,
        chunk_width: 4,
        chunk_height: 4,
        bits_per_sample: 8,
        samples_per_pixel: 1,
        planar_configuration: PlanarConfiguration::Chunky,
    };
    #[rustfmt::skip]
    const RES: [u8;16] = [
        0,1, 2,3,
        1,0, 1,2,

        2,1, 0,1,
        3,2, 1,0,
        ];
    #[rustfmt::skip]
    const RES_RIGHT: [u8;12] = [
        0,1, 2,
        1,0, 1,

        2,1, 0,
        3,2, 1,
        ];
    #[rustfmt::skip]
    const RES_BOT: [u8;12] = [
        0,1,2, 3,
        1,0,1, 2,

        2,1,0, 1,
        ];
    #[rustfmt::skip]
    const RES_BOT_RIGHT: [u8;9] = [
        0,1, 2,
        1,0, 1,

        2,1, 0,
        ];

    #[test]
    fn test_chunk_width_pixels() {
        let info = PRED_INFO;
        assert_eq!(info.chunks_across(), 2);
        assert_eq!(info.chunks_down(), 2);
        assert_eq!(info.chunk_width_pixels(0).unwrap(), info.chunk_width);
        assert_eq!(info.chunk_width_pixels(1).unwrap(), 3);
        info.chunk_width_pixels(2).unwrap_err();
        assert_eq!(info.chunk_height_pixels(0).unwrap(), info.chunk_height);
        assert_eq!(info.chunk_height_pixels(1).unwrap(), 3);
        info.chunk_height_pixels(2).unwrap_err();
    }

    #[test]
    fn test_output_row_stride() {
        let mut info = PRED_INFO;
        assert_eq!(info.output_row_stride(0).unwrap(), 4);
        assert_eq!(info.output_row_stride(1).unwrap(), 3);
        info.output_row_stride(2).unwrap_err();
        info.samples_per_pixel = 2;
        assert_eq!(info.output_row_stride(0).unwrap(), 8);
        assert_eq!(info.output_row_stride(1).unwrap(), 6);
        info.bits_per_sample = 16;
        assert_eq!(info.output_row_stride(0).unwrap(), 16);
        assert_eq!(info.output_row_stride(1).unwrap(), 12);
        info.planar_configuration = PlanarConfiguration::Planar;
        assert_eq!(info.output_row_stride(0).unwrap(), 8);
        assert_eq!(info.output_row_stride(1).unwrap(), 6);
    }

    #[test]
    fn test_output_rows() {
        let mut info = PRED_INFO;
        info.samples_per_pixel = 2;
        assert_eq!(info.output_rows(0).unwrap(), 4);
        assert_eq!(info.output_rows(1).unwrap(), 3);
        info.output_rows(2).unwrap_err();
        info.planar_configuration = PlanarConfiguration::Planar;
        assert_eq!(info.output_rows(0).unwrap(), 8);
        assert_eq!(info.output_rows(1).unwrap(), 6);
    }

    // #[rustfmt::skip]
    // #[test]
    // fn test_no_predict() {
    //     let cases = [
    //         (0,0, Bytes::from_static(&RES[..]),           Bytes::from_static(&RES[..])          ),
    //         (0,1, Bytes::from_static(&RES_BOT[..]),       Bytes::from_static(&RES_BOT[..])      ),
    //         (1,0, Bytes::from_static(&RES_RIGHT[..]),     Bytes::from_static(&RES_RIGHT[..])    ),
    //         (1,1, Bytes::from_static(&RES_BOT_RIGHT[..]), Bytes::from_static(&RES_BOT_RIGHT[..]))
    //     ];
    //     for (x,y, input, expected) in cases {
    //         assert_eq!(fix_endianness(input, &PRED_INFO, x, y).unwrap(), expected);
    //     }
    // }

    #[rustfmt::skip]
    #[test]
    fn test_hdiff_unpredict() {
        let mut predictor_info = PRED_INFO;
        let cases = [
            (0,0, vec![
                0i32, 1, 1, 1,
                1,-1, 1, 1,
                2,-1,-1, 1,
                3,-1,-1,-1,
            ], Vec::from(&RES[..])),
            (0,1, vec![
                0, 1, 1, 1,
                1,-1, 1, 1,
                2,-1,-1, 1,
            ], Vec::from(&RES_BOT[..])),
            (1,0, vec![
                0, 1, 1,
                1,-1, 1,
                2,-1,-1,
                3,-1,-1,
            ], Vec::from(&RES_RIGHT[..])),
            (1,1, vec![
                0, 1, 1,
                1,-1, 1,
                2,-1,-1,
            ], Vec::from(&RES_BOT_RIGHT[..])),
        ];
        for (x,_y, input, expected) in cases {
            println!("uints littleendian");
            predictor_info.endianness = Endianness::LittleEndian;
            predictor_info.bits_per_sample = 8;
            assert_eq!(-1i32 as u8, 255);
            println!("testing u8");
            let buffer = Bytes::from(input.iter().map(|v| *v as u8).collect::<Vec<_>>());
            let res = Bytes::from(expected.clone());
            assert_eq!(unpredict_hdiff(buffer, &predictor_info, x).unwrap(), res);
            assert_eq!(-1i32 as u16, u16::MAX);
            println!("testing u16");
            predictor_info.bits_per_sample = 16;
            let buffer = Bytes::from(input.iter().flat_map(|v| (*v as u16).to_le_bytes()).collect::<Vec<_>>());
            let res = Bytes::from(expected.iter().flat_map(|v| (*v as u16).to_ne_bytes()).collect::<Vec<_>>());
            assert_eq!(unpredict_hdiff(buffer, &predictor_info, x).unwrap(), res);
            assert_eq!(-1i32 as u32, u32::MAX);
            println!("testing u32");
            predictor_info.bits_per_sample = 32;
            let buffer = Bytes::from(input.iter().flat_map(|v| (*v as u32).to_le_bytes()).collect::<Vec<_>>());
            let res = Bytes::from(expected.iter().flat_map(|v| (*v as u32).to_ne_bytes()).collect::<Vec<_>>());
            assert_eq!(unpredict_hdiff(buffer, &predictor_info, x).unwrap(), res);
            assert_eq!(-1i32 as u64, u64::MAX);
            println!("testing u64");
            predictor_info.bits_per_sample = 64;
            let buffer = Bytes::from(input.iter().flat_map(|v| (*v as u64).to_le_bytes()).collect::<Vec<_>>());
            let res = Bytes::from(expected.iter().flat_map(|v| (*v as u64).to_ne_bytes()).collect::<Vec<_>>());
            assert_eq!(unpredict_hdiff(buffer, &predictor_info, x).unwrap(), res);

            println!("ints littleendian");
            predictor_info.bits_per_sample = 8;
            println!("testing i8");
            let buffer = Bytes::from(input.iter().flat_map(|v| (*v as i8).to_le_bytes()).collect::<Vec<_>>());
            println!("{:?}", &buffer[..]);
            let res = Bytes::from(expected.clone());
            assert_eq!(unpredict_hdiff(buffer, &predictor_info, x).unwrap()[..], res[..]);
            println!("testing i16");
            predictor_info.bits_per_sample = 16;
            let buffer = Bytes::from(input.iter().flat_map(|v| (*v as i16).to_le_bytes()).collect::<Vec<_>>());
            let res = Bytes::from(expected.iter().flat_map(|v| (*v as i16).to_ne_bytes()).collect::<Vec<_>>());
            assert_eq!(unpredict_hdiff(buffer, &predictor_info, x).unwrap(), res);
            println!("testing i32");
            predictor_info.bits_per_sample = 32;
            let buffer = Bytes::from(input.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>());
            let res = Bytes::from(expected.iter().flat_map(|v| (*v as i32).to_ne_bytes()).collect::<Vec<_>>());
            assert_eq!(unpredict_hdiff(buffer, &predictor_info, x).unwrap(), res);
            println!("testing i64");
            predictor_info.bits_per_sample = 64;
            let buffer = Bytes::from(input.iter().flat_map(|v| (*v as i64).to_le_bytes()).collect::<Vec<_>>());
            let res = Bytes::from(expected.iter().flat_map(|v| (*v as i64).to_ne_bytes()).collect::<Vec<_>>());
            assert_eq!(unpredict_hdiff(buffer, &predictor_info, x).unwrap(), res);

            println!("uints bigendian");
            predictor_info.endianness = Endianness::BigEndian;
            predictor_info.bits_per_sample = 8;
            assert_eq!(-1i32 as u8, 255);
            println!("testing u8");
            let buffer = Bytes::from(input.iter().map(|v| *v as u8).collect::<Vec<_>>());
            let res = Bytes::from(expected.clone());
            assert_eq!(unpredict_hdiff(buffer, &predictor_info, x).unwrap(), res);
            assert_eq!(-1i32 as u16, u16::MAX);
            println!("testing u16");
            predictor_info.bits_per_sample = 16;
            let buffer = Bytes::from(input.iter().flat_map(|v| (*v as u16).to_be_bytes()).collect::<Vec<_>>());
            let res = Bytes::from(expected.iter().flat_map(|v| (*v as u16).to_ne_bytes()).collect::<Vec<_>>());
            println!("buffer: {:?}", &buffer[..]);
            assert_eq!(unpredict_hdiff(buffer, &predictor_info, x).unwrap()[..], res[..]);
            assert_eq!(-1i32 as u32, u32::MAX);
            println!("testing u32");
            predictor_info.bits_per_sample = 32;
            let buffer = Bytes::from(input.iter().flat_map(|v| (*v as u32).to_be_bytes()).collect::<Vec<_>>());
            let res = Bytes::from(expected.iter().flat_map(|v| (*v as u32).to_ne_bytes()).collect::<Vec<_>>());
            assert_eq!(unpredict_hdiff(buffer, &predictor_info, x).unwrap(), res);
            assert_eq!(-1i32 as u64, u64::MAX);
            println!("testing u64");
            predictor_info.bits_per_sample = 64;
            let buffer = Bytes::from(input.iter().flat_map(|v| (*v as u64).to_be_bytes()).collect::<Vec<_>>());
            let res = Bytes::from(expected.iter().flat_map(|v| (*v as u64).to_ne_bytes()).collect::<Vec<_>>());
            assert_eq!(unpredict_hdiff(buffer, &predictor_info, x).unwrap(), res);

            println!("ints bigendian");
            predictor_info.bits_per_sample = 8;
            assert_eq!(-1i32 as u8, 255);
            println!("testing i8");
            let buffer = Bytes::from(input.iter().flat_map(|v| (*v as i8).to_be_bytes()).collect::<Vec<_>>());
            let res = Bytes::from(expected.clone());
            assert_eq!(unpredict_hdiff(buffer, &predictor_info, x).unwrap(), res);
            assert_eq!(-1i32 as u16, u16::MAX);
            println!("testing i16");
            predictor_info.bits_per_sample = 16;
            let buffer = Bytes::from(input.iter().flat_map(|v| (*v as i16).to_be_bytes()).collect::<Vec<_>>());
            let res = Bytes::from(expected.iter().flat_map(|v| (*v as i16).to_ne_bytes()).collect::<Vec<_>>());
            assert_eq!(unpredict_hdiff(buffer, &predictor_info, x).unwrap(), res);
            assert_eq!(-1i32 as u32, u32::MAX);
            println!("testing i32");
            predictor_info.bits_per_sample = 32;
            let buffer = Bytes::from(input.iter().flat_map(|v| v.to_be_bytes()).collect::<Vec<_>>());
            let res = Bytes::from(expected.iter().flat_map(|v| (*v as i32).to_ne_bytes()).collect::<Vec<_>>());
            assert_eq!(unpredict_hdiff(buffer, &predictor_info, x).unwrap(), res);
            assert_eq!(-1i32 as u64, u64::MAX);
            println!("testing i64");
            predictor_info.bits_per_sample = 64;
            let buffer = Bytes::from(input.iter().flat_map(|v| (*v as i64).to_be_bytes()).collect::<Vec<_>>());
            let res = Bytes::from(expected.iter().flat_map(|v| (*v as i64).to_ne_bytes()).collect::<Vec<_>>());
            assert_eq!(unpredict_hdiff(buffer, &predictor_info, x).unwrap(), res);
        }
    }

    #[rustfmt::skip]
    #[test]
    fn test_predict_f16() {
        // take a 4-value image
        let expect_le = [1,0,3,2,5,4,7,6u8];
        let _expected = [0,1,2,3,4,5,6,7u8];
        //                              0       1
        //                            0       1
        //                          0       1
        //                        0       1
        let _shuffled = [0,2,4,6,1,3,5,7u8];
        let diffed = [0,2,2,2,251,2,2,2];
        let info = PredictorInfo {
            endianness: Endianness::LittleEndian,
            image_width: 4+4,
            image_height: 4+1,
            chunk_width: 4,
            chunk_height: 4,
            bits_per_sample: 16,
            samples_per_pixel: 1,
            planar_configuration: PlanarConfiguration::Chunky,
        };
        let input = Bytes::from_owner(diffed);
        assert_eq!(
            &unpredict_float(input, &info, 1, 1).unwrap()[..],
            &expect_le[..]
        )
    }

    #[rustfmt::skip]
    #[test]
    fn test_predict_f16_padding() {
        // take a 4-pixel image with 2 padding pixels
        let expect_le = [1,0,3,2u8]; // no padding
        let _expected = [0,1,2,3,0,0,0,0u8]; //padding added
        //                              0       1
        //                            0       1
        //                          0       1
        //                        0       1
        let _shuffled = [0,2,0,0,1,3,0,0u8];
        let diffed = [0,2,254,0,1,2,253,0];
        let info = PredictorInfo {
            endianness: Endianness::LittleEndian,
            image_width: 4+2,
            image_height: 4+1,
            chunk_width: 4,
            chunk_height: 4,
            bits_per_sample: 16,
            samples_per_pixel: 1,
            planar_configuration: PlanarConfiguration::Chunky,
        };
        let input = Bytes::from_owner(diffed);
        assert_eq!(
            &unpredict_float(input, &info, 1, 1).unwrap()[..],
            &expect_le[..]
        )
    }

    #[rustfmt::skip]
    #[test]
    fn test_fpredict_f32() {
        // let's take this 2-value image where we only look at bytes
        let expect_le  = [3,2,  1,0,  7,6,  5,4];
        let _expected  = [0,1,  2,3,  4,5,  6,7u8];
        //                           0     1     2     3   \_ de-shuffling indices
        //                         0     1     2     3     /  (the one the function uses)
        let _shuffled  = [0,4,  1,5,  2,6,  3,7u8];
        let diffed     = [0,4,253,4,253,4,253,4u8];
        println!("expected: {expect_le:?}");
        let info = PredictorInfo {
            endianness: Endianness::LittleEndian,
            image_width: 2,
            image_height: 2 + 1,
            chunk_width: 2,
            chunk_height: 2,
            bits_per_sample: 32,
            samples_per_pixel: 1,
            planar_configuration: PlanarConfiguration::Chunky,
        };
        let input = Bytes::from_owner(diffed);
        assert_eq!(
            &unpredict_float(input.clone(), &info, 0, 1).unwrap()[..],
            &expect_le
        );
    }

    #[rustfmt::skip]
    #[test]
    fn test_fpredict_f64() {
        assert_eq!(f64::from_le_bytes([7,6,5,4,3,2,1,0]), f64::from_bits(0x00_01_02_03_04_05_06_07));
        // let's take this 2-value image
        let expect_be =  [7,6,5,4,3, 2,1, 0,15,14,13,12,11,10,9,8];
        let _expected  = [0,1,2,3,4, 5,6, 7,8, 9,10,11,12,13,14,15u8];
        //                           0   1    2    3    4     5     6     7
        //                         0   1   2    3    4     5     6     7
        let _shuffled = [0,8,1,9,2,10,3,11,4,12, 5,13, 6,14, 7,15u8];
        let diffed = [0,8,249,8,249,8,249,8,249,8,249,8,249,8,249,8u8];
        let info = PredictorInfo {
            endianness: Endianness::LittleEndian,
            image_width: 2,
            image_height: 2 + 1,
            chunk_width: 2,
            chunk_height: 2,
            bits_per_sample: 64,
            samples_per_pixel: 1,
            planar_configuration: PlanarConfiguration::Chunky,
        };
        let input = Bytes::from_owner(diffed);
        assert_eq!(
            &unpredict_float(input, &info, 0, 1)
                .unwrap()[..],
            &expect_be[..]
        );
    }
}
