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
            9..=16 => buffer
                .as_chunks_mut::<2>()
                .0
                .iter_mut()
                .for_each(|v| *v = u16::from_le_bytes(*v).to_ne_bytes()),
            17..=32 => buffer
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .for_each(|v| *v = u32::from_le_bytes(*v).to_ne_bytes()),
            _ => buffer
                .as_chunks_mut::<8>()
                .0
                .iter_mut()
                .for_each(|v| *v = u64::from_le_bytes(*v).to_ne_bytes()),
        },
        Endianness::BigEndian => match bits_per_sample {
            0..=8 => {}
            9..=16 => buffer
                .as_chunks_mut::<2>()
                .0
                .iter_mut()
                .for_each(|v| *v = u16::from_be_bytes(*v).to_ne_bytes()),
            17..=32 => buffer
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .for_each(|v| *v = u32::from_be_bytes(*v).to_ne_bytes()),
            _ => buffer
                .as_chunks_mut::<8>()
                .0
                .iter_mut()
                .for_each(|v| *v = u64::from_be_bytes(*v).to_ne_bytes()),
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
    for (i, chunk) in output.as_chunks_mut::<2>().0.iter_mut().enumerate() {
        *chunk = u16::from_be_bytes([input[i], input[input.len() / 2 + i]]).to_ne_bytes();
    }
}

fn rev_predict_f32(input: &mut [u8], output: &mut [u8], samples: usize) {
    for i in samples..input.len() {
        input[i] = input[i].wrapping_add(input[i - samples]);
    }
    for (i, chunk) in output.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        *chunk = u32::from_be_bytes([
            input[i],
            input[input.len() / 4 + i],
            input[input.len() / 4 * 2 + i],
            input[input.len() / 4 * 3 + i],
        ])
        .to_ne_bytes();
    }
}

fn rev_predict_f64(input: &mut [u8], output: &mut [u8], samples: usize) {
    for i in samples..input.len() {
        input[i] = input[i].wrapping_add(input[i - samples]);
    }
    for (i, chunk) in output.as_chunks_mut::<8>().0.iter_mut().enumerate() {
        *chunk = u64::from_be_bytes([
            input[i],
            input[input.len() / 8 + i],
            input[input.len() / 8 * 2 + i],
            input[input.len() / 8 * 3 + i],
            input[input.len() / 8 * 4 + i],
            input[input.len() / 8 * 5 + i],
            input[input.len() / 8 * 6 + i],
            input[input.len() / 8 * 7 + i],
        ])
        .to_ne_bytes();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NATIVE: Endianness = if cfg!(target_endian = "little") {
        Endianness::LittleEndian
    } else {
        Endianness::BigEndian
    };

    /// Deterministic pseudo-random `bits`-bit values (xorshift64), so sums wrap and carry.
    fn random_values(count: usize, bits: u16) -> Vec<u64> {
        let mut state: u64 = 0x2545_F491_4F6C_DD1D;
        (0..count)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state >> (64 - u32::from(bits))
            })
            .collect()
    }

    /// The low `size` bytes of `value`, in the given byte order.
    fn to_bytes(value: u64, size: usize, endianness: Endianness) -> Vec<u8> {
        match endianness {
            Endianness::LittleEndian => value.to_le_bytes()[..size].to_vec(),
            Endianness::BigEndian => value.to_be_bytes()[8 - size..].to_vec(),
        }
    }

    /// `values` as `size`-byte native-endian samples: what both predictors should return.
    fn native(values: &[u64], size: usize) -> Vec<u8> {
        values
            .iter()
            .flat_map(|v| to_bytes(*v, size, NATIVE))
            .collect()
    }

    /// Horizontal differencing as a TIFF writer applies it: each sample minus the same sample of
    /// the previous pixel in its row, stored in the file's byte order.
    fn hdiff_encode(
        values: &[u64],
        endianness: Endianness,
        samples: usize,
        size: usize,
        tile_width: usize,
    ) -> Vec<u8> {
        values
            .chunks(tile_width * samples)
            .flat_map(|row| {
                row.iter()
                    .enumerate()
                    .map(move |(i, value)| match i.checked_sub(samples) {
                        Some(prev) => value.wrapping_sub(row[prev]),
                        None => *value,
                    })
            })
            .flat_map(|delta| to_bytes(delta, size, endianness))
            .collect()
    }

    /// The floating point predictor as a TIFF writer applies it: split each row into byte planes,
    /// most significant byte first, then difference each byte from the one `samples` before it.
    fn float_encode(values: &[u64], samples: usize, size: usize, tile_width: usize) -> Vec<u8> {
        values
            .chunks(tile_width * samples)
            .flat_map(|row| {
                let big_endian: Vec<Vec<u8>> = row
                    .iter()
                    .map(|v| to_bytes(*v, size, Endianness::BigEndian))
                    .collect();
                let mut bytes: Vec<u8> = (0..size)
                    .flat_map(|plane| big_endian.iter().map(move |b| b[plane]))
                    .collect();
                for i in (samples..bytes.len()).rev() {
                    bytes[i] = bytes[i].wrapping_sub(bytes[i - samples]);
                }
                bytes
            })
            .collect()
    }

    #[test]
    fn test_hdiff_round_trip() {
        let rows = 3;
        for bits_per_sample in [8, 16, 32, 64] {
            let size = usize::from(bits_per_sample / 8);
            // 1 to 4 samples per pixel have dedicated loops; 5 and 6 take the general path.
            for samples in 1..=6 {
                for tile_width in [1, 2, 7, 33] {
                    let values = random_values(rows * tile_width * samples, bits_per_sample);
                    for endianness in [Endianness::LittleEndian, Endianness::BigEndian] {
                        let encoded = hdiff_encode(&values, endianness, samples, size, tile_width);
                        let decoded = unpredict_hdiff(
                            encoded,
                            endianness,
                            samples,
                            bits_per_sample,
                            tile_width,
                        );
                        assert_eq!(
                            decoded,
                            native(&values, size),
                            "{bits_per_sample}-bit, {samples} samples, width {tile_width}, {endianness:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_float_round_trip() {
        let rows = 3;
        for bits_per_sample in [16, 32, 64] {
            let size = usize::from(bits_per_sample / 8);
            for samples in 1..=6 {
                for tile_width in [1, 2, 7, 33] {
                    let values = random_values(rows * tile_width * samples, bits_per_sample);
                    let encoded = float_encode(&values, samples, size, tile_width);
                    let decoded =
                        unpredict_float(encoded, samples, bits_per_sample, tile_width).unwrap();
                    assert_eq!(
                        decoded,
                        native(&values, size),
                        "{bits_per_sample}-bit, {samples} samples, width {tile_width}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_hdiff_adds_same_sample_of_previous_pixel() {
        // Two rows of two RGB pixels. Each band sums on its own, wraps at 256, and starts over
        // on the next row.
        let encoded = vec![10, 20, 250, 1, 2, 10, 5, 6, 7, 1, 1, 1];
        let decoded = unpredict_hdiff(encoded, Endianness::LittleEndian, 3, 8, 2);
        assert_eq!(decoded, vec![10, 20, 250, 11, 22, 4, 5, 6, 7, 6, 7, 8]);
    }

    #[test]
    fn test_hdiff_swaps_big_endian_samples_before_adding() {
        // 0x00FF + 0x0001 carries into the high byte, which only works after swapping.
        let encoded = vec![0x00, 0xFF, 0x00, 0x01];
        let decoded = unpredict_hdiff(encoded, Endianness::BigEndian, 1, 16, 2);
        let expected: Vec<u8> = [0x00FF_u16, 0x0100]
            .iter()
            .flat_map(|v| v.to_ne_bytes())
            .collect();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_float_reassembles_byte_planes() {
        // 1.0 and 2.0 as f32 are 3F800000 and 40000000. Split into byte planes, most significant
        // first, they are [3F 40] [80 00] [00 00] [00 00]; differencing each byte from the one
        // before it gives:
        let encoded = vec![0x3F, 0x01, 0x40, 0x80, 0x00, 0x00, 0x00, 0x00];
        let decoded = unpredict_float(encoded, 1, 32, 2).unwrap();
        let expected: Vec<u8> = [1.0_f32, 2.0]
            .iter()
            .flat_map(|v| v.to_ne_bytes())
            .collect();
        assert_eq!(decoded, expected);
    }

    #[test]
    fn test_float_rejects_unsupported_bit_depth() {
        assert!(unpredict_float(vec![0; 6], 1, 24, 2).is_err());
    }
}
