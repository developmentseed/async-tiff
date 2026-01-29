use crate::tags::SampleFormat;

/// Supported numeric data types for array elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataType {
    /// Boolean mask data.
    Bool,
    /// Unsigned 8-bit integer.
    UInt8,
    /// Unsigned 16-bit integer.
    UInt16,
    /// Unsigned 32-bit integer.
    UInt32,
    /// Unsigned 64-bit integer.
    UInt64,
    /// Signed 8-bit integer.
    Int8,
    /// Signed 16-bit integer.
    Int16,
    /// Signed 32-bit integer.
    Int32,
    /// Signed 64-bit integer.
    Int64,
    /// 32-bit floating point.
    Float32,
    /// 64-bit floating point.
    Float64,
}

impl DataType {
    /// The size in bytes of this data type.
    ///
    /// ```
    /// use async_tiff::DataType;
    ///
    /// assert_eq!(DataType::Bool.size(), 1);
    /// assert_eq!(DataType::UInt8.size(), 1);
    /// assert_eq!(DataType::Int16.size(), 2);
    /// assert_eq!(DataType::Float32.size(), 4);
    /// assert_eq!(DataType::Float64.size(), 8);
    /// ```
    pub fn size(&self) -> usize {
        match self {
            DataType::Bool | DataType::UInt8 | DataType::Int8 => 1,
            DataType::UInt16 | DataType::Int16 => 2,
            DataType::UInt32 | DataType::Int32 | DataType::Float32 => 4,
            DataType::UInt64 | DataType::Int64 | DataType::Float64 => 8,
        }
    }

    /// Parse a DataType from TIFF IFD tags.
    ///
    /// Returns `None` if the combination of sample format and bits per sample
    /// is not supported, or if the values are inconsistent across samples.
    ///
    /// # Arguments
    /// * `sample_format` - The SampleFormat values from the TIFF IFD
    /// * `bits_per_sample` - The BitsPerSample values from the TIFF IFD
    pub(crate) fn from_tags(
        sample_format: &[SampleFormat],
        bits_per_sample: &[u16],
    ) -> Option<Self> {
        // All samples must have the same format and bit depth
        let first_format = sample_format.first()?;
        let first_bits = bits_per_sample.first()?;

        // Check that all samples have consistent format and bit depth
        if !sample_format.iter().all(|f| f == first_format) {
            return None;
        }
        if !bits_per_sample.iter().all(|b| b == first_bits) {
            return None;
        }

        match (first_format, first_bits) {
            (SampleFormat::Uint, 1) => Some(DataType::Bool),
            (SampleFormat::Uint, 8) => Some(DataType::UInt8),
            (SampleFormat::Uint, 16) => Some(DataType::UInt16),
            (SampleFormat::Uint, 32) => Some(DataType::UInt32),
            (SampleFormat::Uint, 64) => Some(DataType::UInt64),
            (SampleFormat::Int, 8) => Some(DataType::Int8),
            (SampleFormat::Int, 16) => Some(DataType::Int16),
            (SampleFormat::Int, 32) => Some(DataType::Int32),
            (SampleFormat::Int, 64) => Some(DataType::Int64),
            (SampleFormat::Float, 32) => Some(DataType::Float32),
            (SampleFormat::Float, 64) => Some(DataType::Float64),
            // Unsupported combinations (e.g., Void, Unknown, or unusual bit depths)
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_tags_uint_types() {
        assert_eq!(
            DataType::from_tags(&[SampleFormat::Uint], &[1]),
            Some(DataType::Bool),
            "Uint 1-bit should be Bool"
        );
        assert_eq!(
            DataType::from_tags(&[SampleFormat::Uint], &[8]),
            Some(DataType::UInt8),
            "Uint 8-bit should be UInt8"
        );
        assert_eq!(
            DataType::from_tags(&[SampleFormat::Uint], &[16]),
            Some(DataType::UInt16),
            "Uint 16-bit should be UInt16"
        );
        assert_eq!(
            DataType::from_tags(&[SampleFormat::Uint], &[32]),
            Some(DataType::UInt32),
            "Uint 32-bit should be UInt32"
        );
        assert_eq!(
            DataType::from_tags(&[SampleFormat::Uint], &[64]),
            Some(DataType::UInt64),
            "Uint 64-bit should be UInt64"
        );
    }

    #[test]
    fn test_from_tags_int_types() {
        assert_eq!(
            DataType::from_tags(&[SampleFormat::Int], &[8]),
            Some(DataType::Int8),
            "Int 8-bit should be Int8"
        );
        assert_eq!(
            DataType::from_tags(&[SampleFormat::Int], &[16]),
            Some(DataType::Int16),
            "Int 16-bit should be Int16"
        );
        assert_eq!(
            DataType::from_tags(&[SampleFormat::Int], &[32]),
            Some(DataType::Int32),
            "Int 32-bit should be Int32"
        );
        assert_eq!(
            DataType::from_tags(&[SampleFormat::Int], &[64]),
            Some(DataType::Int64),
            "Int 64-bit should be Int64"
        );
    }

    #[test]
    fn test_from_tags_float_types() {
        assert_eq!(
            DataType::from_tags(&[SampleFormat::Float], &[32]),
            Some(DataType::Float32),
            "Float 32-bit should be Float32"
        );
        assert_eq!(
            DataType::from_tags(&[SampleFormat::Float], &[64]),
            Some(DataType::Float64),
            "Float 64-bit should be Float64"
        );
    }

    #[test]
    fn test_from_tags_rgb_consistent() {
        // RGB image with 3 samples, all UInt8
        assert_eq!(
            DataType::from_tags(
                &[SampleFormat::Uint, SampleFormat::Uint, SampleFormat::Uint],
                &[8, 8, 8]
            ),
            Some(DataType::UInt8),
            "RGB with consistent UInt8 should succeed"
        );

        // RGB image with 3 samples, all UInt16
        assert_eq!(
            DataType::from_tags(
                &[SampleFormat::Uint, SampleFormat::Uint, SampleFormat::Uint],
                &[16, 16, 16]
            ),
            Some(DataType::UInt16),
            "RGB with consistent UInt16 should succeed"
        );
    }

    #[test]
    fn test_from_tags_inconsistent_format() {
        // Mixed formats should return None
        assert_eq!(
            DataType::from_tags(&[SampleFormat::Uint, SampleFormat::Int], &[8, 8]),
            None,
            "Inconsistent sample formats should return None"
        );
    }

    #[test]
    fn test_from_tags_inconsistent_bits() {
        // Mixed bit depths should return None
        assert_eq!(
            DataType::from_tags(&[SampleFormat::Uint, SampleFormat::Uint], &[8, 16]),
            None,
            "Inconsistent bit depths should return None"
        );
    }

    #[test]
    fn test_from_tags_empty_arrays() {
        assert_eq!(
            DataType::from_tags(&[], &[]),
            None,
            "Empty arrays should return None"
        );
    }

    #[test]
    fn test_from_tags_unsupported_bit_depth() {
        // Unsupported bit depth
        assert_eq!(
            DataType::from_tags(&[SampleFormat::Uint], &[12]),
            None,
            "Unsupported bit depth (12) should return None"
        );
        assert_eq!(
            DataType::from_tags(&[SampleFormat::Uint], &[24]),
            None,
            "Unsupported bit depth (24) should return None"
        );
    }

    #[test]
    fn test_from_tags_unsupported_format() {
        // Void format is not supported
        assert_eq!(
            DataType::from_tags(&[SampleFormat::Void], &[8]),
            None,
            "Void format should return None"
        );

        // Unknown format should also return None
        assert_eq!(
            DataType::from_tags(&[SampleFormat::Unknown(99)], &[8]),
            None,
            "Unknown format should return None"
        );
    }
}
