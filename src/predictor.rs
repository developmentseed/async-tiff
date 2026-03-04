//! TIFF predictor implementations for reversing lossless pre-compression transforms.
//!
//! Predictors operate on the **full encoded tile width** — never the cropped edge-tile width.
//! Cropping to valid pixels happens downstream (in the caller) after prediction.

use crate::error::{AsyncTiffError, AsyncTiffResult};
use crate::reader::Endianness;

/// Fix endianness in-place. If `byte_order` matches the host, this is a no-op.
pub(crate) fn fix_endianness(buffer: &mut [u8], byte_order: Endianness, bits_per_sample: u16) {
    #[cfg(target_endian = "little")]
    if let Endianness::LittleEndian = byte_order {
        return;
    }
    #[cfg(target_endian = "big")]
    if let Endianness::BigEndian = byte_order {
        return;
    }

    match byte_order {
        Endianness::LittleEndian => match bits_per_sample {
            0..=8 => {}
            9..=16 => buffer.chunks_exact_mut(2).for_each(|v| {
                v.copy_from_slice(&u16::from_le_bytes((*v).try_into().unwrap()).to_ne_bytes())
            }),
            17..=32 => buffer.chunks_exact_mut(4).for_each(|v| {
                v.copy_from_slice(&u32::from_le_bytes((*v).try_into().unwrap()).to_ne_bytes())
            }),
            _ => buffer.chunks_exact_mut(8).for_each(|v| {
                v.copy_from_slice(&u64::from_le_bytes((*v).try_into().unwrap()).to_ne_bytes())
            }),
        },
        Endianness::BigEndian => match bits_per_sample {
            0..=8 => {}
            9..=16 => buffer.chunks_exact_mut(2).for_each(|v| {
                v.copy_from_slice(&u16::from_be_bytes((*v).try_into().unwrap()).to_ne_bytes())
            }),
            17..=32 => buffer.chunks_exact_mut(4).for_each(|v| {
                v.copy_from_slice(&u32::from_be_bytes((*v).try_into().unwrap()).to_ne_bytes())
            }),
            _ => buffer.chunks_exact_mut(8).for_each(|v| {
                v.copy_from_slice(&u64::from_be_bytes((*v).try_into().unwrap()).to_ne_bytes())
            }),
        },
    }
}

/// Reverse horizontal differencing predictor (Predictor=2).
///
/// Operates on the **full encoded tile width** — `tile_width` must be the nominal tile width,
/// not the cropped valid width of an edge tile.
///
/// Fixes endianness first, then reverses the per-sample delta encoding.
pub(crate) fn unpredict_hdiff(
    mut buffer: Vec<u8>,
    endianness: Endianness,
    samples: usize,
    bits_per_sample: u16,
    tile_width: usize,
) -> Vec<u8> {
    let bytes_per_sample = (bits_per_sample as usize).div_ceil(8);
    let row_stride = tile_width * samples * bytes_per_sample;

    fix_endianness(&mut buffer, endianness, bits_per_sample);

    for row in buffer.chunks_mut(row_stride) {
        rev_hpredict_row(row, bits_per_sample, samples);
    }

    buffer
}

/// Reverse one row of horizontal differencing, dispatched by bit depth.
fn rev_hpredict_row(row: &mut [u8], bits_per_sample: u16, samples: usize) {
    match bits_per_sample {
        0..=8 => {
            for i in samples..row.len() {
                row[i] = row[i].wrapping_add(row[i - samples]);
            }
        }
        9..=16 => {
            for i in (samples * 2..row.len()).step_by(2) {
                let v = u16::from_ne_bytes(row[i..][..2].try_into().unwrap());
                let p = u16::from_ne_bytes(row[i - 2 * samples..][..2].try_into().unwrap());
                row[i..][..2].copy_from_slice(&v.wrapping_add(p).to_ne_bytes());
            }
        }
        17..=32 => {
            for i in (samples * 4..row.len()).step_by(4) {
                let v = u32::from_ne_bytes(row[i..][..4].try_into().unwrap());
                let p = u32::from_ne_bytes(row[i - 4 * samples..][..4].try_into().unwrap());
                row[i..][..4].copy_from_slice(&v.wrapping_add(p).to_ne_bytes());
            }
        }
        33..=64 => {
            for i in (samples * 8..row.len()).step_by(8) {
                let v = u64::from_ne_bytes(row[i..][..8].try_into().unwrap());
                let p = u64::from_ne_bytes(row[i - 8 * samples..][..8].try_into().unwrap());
                row[i..][..8].copy_from_slice(&v.wrapping_add(p).to_ne_bytes());
            }
        }
        _ => unreachable!("unsupported bits_per_sample {bits_per_sample}"),
    }
}

/// Reverse floating-point predictor (Predictor=3).
///
/// Operates on the **full encoded tile width** — `tile_width` must be the nominal tile width,
/// not the cropped valid width of an edge tile.
///
/// Returns a buffer the same size as the input. The caller is responsible for cropping
/// edge tiles afterward.
///
/// Per the TIFF floating-point predictor spec, no external byte-order fixup is applied.
pub(crate) fn unpredict_float(
    mut buffer: Vec<u8>,
    samples: usize,
    bits_per_sample: u16,
    tile_width: usize,
) -> AsyncTiffResult<Vec<u8>> {
    let bytes_per_sample = (bits_per_sample as usize) / 8;
    let row_stride = tile_width * samples * bytes_per_sample;
    let mut out = vec![0u8; buffer.len()];

    for (in_row, out_row) in buffer
        .chunks_mut(row_stride)
        .zip(out.chunks_mut(row_stride))
    {
        match bits_per_sample {
            16 => rev_predict_f16(in_row, out_row, samples),
            32 => rev_predict_f32(in_row, out_row, samples),
            64 => rev_predict_f64(in_row, out_row, samples),
            _ => {
                return Err(AsyncTiffError::General(format!(
                    "Floating-point predictor not supported for {bits_per_sample}-bit samples"
                )))
            }
        }
    }

    Ok(out)
}

fn rev_predict_f16(input: &mut [u8], output: &mut [u8], samples: usize) {
    for i in samples..input.len() {
        input[i] = input[i].wrapping_add(input[i - samples]);
    }
    for (i, chunk) in output.chunks_exact_mut(2).enumerate() {
        chunk.copy_from_slice(&u16::to_ne_bytes(u16::from_be_bytes([
            input[i],
            input[input.len() / 2 + i],
        ])));
    }
}

fn rev_predict_f32(input: &mut [u8], output: &mut [u8], samples: usize) {
    for i in samples..input.len() {
        input[i] = input[i].wrapping_add(input[i - samples]);
    }
    for (i, chunk) in output.chunks_exact_mut(4).enumerate() {
        chunk.copy_from_slice(&u32::to_ne_bytes(u32::from_be_bytes([
            input[i],
            input[input.len() / 4 + i],
            input[input.len() / 4 * 2 + i],
            input[input.len() / 4 * 3 + i],
        ])));
    }
}

fn rev_predict_f64(input: &mut [u8], output: &mut [u8], samples: usize) {
    for i in samples..input.len() {
        input[i] = input[i].wrapping_add(input[i - samples]);
    }
    for (i, chunk) in output.chunks_exact_mut(8).enumerate() {
        chunk.copy_from_slice(&u64::to_ne_bytes(u64::from_be_bytes([
            input[i],
            input[input.len() / 8 + i],
            input[input.len() / 8 * 2 + i],
            input[input.len() / 8 * 3 + i],
            input[input.len() / 8 * 4 + i],
            input[input.len() / 8 * 5 + i],
            input[input.len() / 8 * 6 + i],
            input[input.len() / 8 * 7 + i],
        ])));
    }
}
