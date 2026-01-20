use crate::tags::SampleFormat;

/// Supported numeric data types for array elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataType {
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
    pub fn size(&self) -> usize {
        match self {
            DataType::UInt8 | DataType::Int8 => 1,
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
            (SampleFormat::Uint, 8) => Some(DataType::UInt8),
            (SampleFormat::Uint, 16) => Some(DataType::UInt16),
            (SampleFormat::Uint, 32) => Some(DataType::UInt32),
            (SampleFormat::Uint, 64) => Some(DataType::UInt64),
            (SampleFormat::Int, 8) => Some(DataType::Int8),
            (SampleFormat::Int, 16) => Some(DataType::Int16),
            (SampleFormat::Int, 32) => Some(DataType::Int32),
            (SampleFormat::Int, 64) => Some(DataType::Int64),
            (SampleFormat::IEEEFP, 32) => Some(DataType::Float32),
            (SampleFormat::IEEEFP, 64) => Some(DataType::Float64),
            // Unsupported combinations (e.g., Void, Unknown, or unusual bit depths)
            _ => None,
        }
    }
}
