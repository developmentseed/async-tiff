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

    // Swapping in its own pass vectorizes. Swapping inside the summing loop below instead
    // measured slower on aarch64, where it lengthens the loop-carried dependency.
    fix_endianness(&mut buffer, endianness, bits_per_sample);

    match bits_per_sample {
        0..=8 => rev_hpredict::<u8>(&mut buffer, row_stride, samples),
        9..=16 => rev_hpredict::<u16>(&mut buffer, row_stride, samples),
        17..=32 => rev_hpredict::<u32>(&mut buffer, row_stride, samples),
        33..=64 => rev_hpredict::<u64>(&mut buffer, row_stride, samples),
        _ => unreachable!("unsupported bits_per_sample {bits_per_sample}"),
    }

    buffer
}

/// Reverse horizontal differencing on each row of native-endian `T` samples.
fn rev_hpredict<T: Sample>(buffer: &mut [u8], row_stride: usize, samples: usize) {
    for row in buffer.chunks_mut(row_stride) {
        accumulate_row::<T>(T::as_samples_mut(row), samples);
    }
}

/// Reverse horizontal differencing on one row of `samples`-sample pixels.
///
/// Each sample becomes the wrapping sum of itself and every earlier sample of the same channel in
/// the row. Summing in place (`row[i] += row[i - samples]`) reloads the value stored one pixel
/// earlier, so every sample waits on a store-to-load round trip. For one to four samples per
/// pixel, keep one running sum per channel instead: the constant `N` lets LLVM unroll the channel
/// loop and hold all `N` sums in registers. Wider pixels are uncommon and keep the in-place loop.
fn accumulate_row<T: Sample>(row: &mut [T::Bytes], samples: usize) {
    match samples {
        1 => accumulate::<T, 1>(row),
        2 => accumulate::<T, 2>(row),
        3 => accumulate::<T, 3>(row),
        4 => accumulate::<T, 4>(row),
        _ => {
            for i in samples..row.len() {
                let sum = T::from_ne_bytes(row[i]).wrapping_add(T::from_ne_bytes(row[i - samples]));
                row[i] = sum.to_ne_bytes();
            }
        }
    }
}

fn accumulate<T: Sample, const N: usize>(row: &mut [T::Bytes]) {
    let mut sums = [T::ZERO; N];
    // Only a truncated tile can end mid-pixel, and decoding rejects its length afterwards, so the
    // partial pixel is left as is.
    for pixel in row.as_chunks_mut::<N>().0 {
        for (sum, sample) in sums.iter_mut().zip(pixel) {
            *sum = sum.wrapping_add(T::from_ne_bytes(*sample));
            *sample = sum.to_ne_bytes();
        }
    }
}

/// An unsigned integer sample, as summed by the horizontal predictor.
trait Sample: Copy {
    /// The sample as stored in a byte buffer, e.g. `[u8; 2]` for `u16`.
    type Bytes: Copy;

    const ZERO: Self;

    /// View `bytes` as whole samples, ignoring any trailing partial sample.
    fn as_samples_mut(bytes: &mut [u8]) -> &mut [Self::Bytes];
    fn from_ne_bytes(bytes: Self::Bytes) -> Self;
    fn to_ne_bytes(self) -> Self::Bytes;
    fn wrapping_add(self, rhs: Self) -> Self;
}

macro_rules! impl_sample {
    ($($ty:ty),*) => {$(
        impl Sample for $ty {
            type Bytes = [u8; size_of::<$ty>()];

            const ZERO: Self = 0;

            fn as_samples_mut(bytes: &mut [u8]) -> &mut [Self::Bytes] {
                bytes.as_chunks_mut().0
            }

            fn from_ne_bytes(bytes: Self::Bytes) -> Self {
                <$ty>::from_ne_bytes(bytes)
            }

            fn to_ne_bytes(self) -> Self::Bytes {
                <$ty>::to_ne_bytes(self)
            }

            fn wrapping_add(self, rhs: Self) -> Self {
                <$ty>::wrapping_add(self, rhs)
            }
        }
    )*};
}

impl_sample!(u8, u16, u32, u64);

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
