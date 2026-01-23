//! Error handling.

use std::error::Error;
use std::fmt::Debug;
use std::{fmt, io, str, string};

use jpeg::UnsupportedFeature;
use thiserror::Error;

use crate::tag_value::TagValue;
use crate::tags::{
    CompressionMethod, PhotometricInterpretation, PlanarConfiguration, Predictor, SampleFormat, Tag,
};

/// Enum with all errors in this crate.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum AsyncTiffError {
    /// End of file error.
    #[error("End of File: expected to read {0} bytes, got {1}")]
    EndOfFile(u64, u64),

    /// General error.
    #[error("General error: {0}")]
    General(String),

    /// Tile index error
    #[error("Tile index out of bounds: {0}, {1}")]
    TileIndexError(u32, u32),

    /// IO Error.
    #[error(transparent)]
    IOError(#[from] std::io::Error),

    /// Error while decoding JPEG data.
    #[error(transparent)]
    JPEGDecodingError(#[from] jpeg::Error),

    /// Error while decoding JPEG2000 data.
    #[cfg(feature = "jpeg2k")]
    #[error(transparent)]
    JPEG2kDecodingError(#[from] jpeg2k::error::Error),

    /// Error while fetching data using object store.
    #[cfg(feature = "object_store")]
    #[error(transparent)]
    ObjectStore(#[from] object_store::Error),

    /// An error during TIFF tag parsing.
    #[error(transparent)]
    InternalTIFFError(#[from] TiffError),

    /// Reqwest error
    #[cfg(feature = "reqwest")]
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    /// External error
    #[error(transparent)]
    External(Box<dyn std::error::Error + Send + Sync>),
}

/// Crate-specific result type.
pub type AsyncTiffResult<T> = std::result::Result<T, AsyncTiffError>;

/// Tiff error kinds.
#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum TiffError {
    /// The Image is not formatted properly.
    FormatError(TiffFormatError),

    /// The Decoder does not support features required by the image.
    UnsupportedError(TiffUnsupportedError),

    /// An I/O Error occurred while decoding the image.
    IoError(io::Error),

    /// An integer conversion to or from a platform size failed, either due to
    /// limits of the platform size or limits of the format.
    IntSizeError,

    /// The image does not support the requested operation
    UsageError(UsageError),
}

/// The image is not formatted properly.
///
/// This indicates that the encoder producing the image might behave incorrectly or that the input
/// file has been corrupted.
///
/// The list of variants may grow to incorporate errors of future features. Matching against this
/// exhaustively is not covered by interface stability guarantees.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
#[expect(missing_docs)]
pub enum TiffFormatError {
    TiffSignatureNotFound,
    TiffSignatureInvalid,
    ImageFileDirectoryNotFound,
    InconsistentSizesEncountered,
    UnexpectedCompressedData {
        actual_bytes: usize,
        required_bytes: usize,
    },
    InconsistentStripSamples {
        actual_samples: usize,
        required_samples: usize,
    },
    InvalidDimensions(u32, u32),
    InvalidTag,
    InvalidTagValueType(Tag),
    RequiredTagNotFound(Tag),
    UnknownPredictor(u16),
    UnknownPlanarConfiguration(u16),
    ByteExpected(TagValue),
    SignedByteExpected(TagValue),
    ShortExpected(TagValue),
    SignedShortExpected(TagValue),
    UnsignedIntegerExpected(TagValue),
    SignedIntegerExpected(TagValue),
    Format(String),
    RequiredTagEmpty(Tag),
    StripTileTagConflict,
    CycleInOffsets,
    SamplesPerPixelIsZero,
}

impl fmt::Display for TiffFormatError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use self::TiffFormatError::*;
        match *self {
            TiffSignatureNotFound => write!(fmt, "TIFF signature not found."),
            TiffSignatureInvalid => write!(fmt, "TIFF signature invalid."),
            ImageFileDirectoryNotFound => write!(fmt, "Image file directory not found."),
            InconsistentSizesEncountered => write!(fmt, "Inconsistent sizes encountered."),
            UnexpectedCompressedData {
                actual_bytes,
                required_bytes,
            } => {
                write!(
                    fmt,
                    "Decompression returned different amount of bytes than expected: got {actual_bytes}, expected {required_bytes}."
                )
            }
            InconsistentStripSamples {
                actual_samples,
                required_samples,
            } => {
                write!(
                    fmt,
                    "Inconsistent elements in strip: got {actual_samples}, expected {required_samples}."
                )
            }
            InvalidDimensions(width, height) => write!(fmt, "Invalid dimensions: {width}x{height}."),
            InvalidTag => write!(fmt, "Image contains invalid tag."),
            InvalidTagValueType(ref tag) => {
                write!(fmt, "Tag `{tag:?}` did not have the expected value type.")
            }
            RequiredTagNotFound(ref tag) => write!(fmt, "Required tag `{tag:?}` not found."),
            UnknownPredictor(ref predictor) => {
                write!(fmt, "Unknown predictor “{predictor}” encountered")
            }
            UnknownPlanarConfiguration(ref planar_config) =>  {
                write!(fmt, "Unknown planar configuration “{planar_config}” encountered")
            }
            ByteExpected(ref val) => write!(fmt, "Expected byte, {val:?} found."),
            SignedByteExpected(ref val) => write!(fmt, "Expected signed byte, {val:?} found."),
            ShortExpected(ref val) => write!(fmt, "Expected short, {val:?} found."),
            SignedShortExpected(ref val) => write!(fmt, "Expected signed short, {val:?} found."),
            UnsignedIntegerExpected(ref val) => {
                write!(fmt, "Expected unsigned integer, {val:?} found.")
            }
            SignedIntegerExpected(ref val) => {
                write!(fmt, "Expected signed integer, {val:?} found.")
            }
            Format(ref val) => write!(fmt, "Invalid format: {val:?}."),
            RequiredTagEmpty(ref val) => write!(fmt, "Required tag {val:?} was empty."),
            StripTileTagConflict => write!(fmt, "File should contain either (StripByteCounts and StripOffsets) or (TileByteCounts and TileOffsets), other combination was found."),
            CycleInOffsets => write!(fmt, "File contained a cycle in the list of IFDs"),
            SamplesPerPixelIsZero => write!(fmt, "Samples per pixel is zero"),
        }
    }
}

/// The Decoder does not support features required by the image.
///
/// This only captures known failures for which the standard either does not require support or an
/// implementation has been planned but not yet completed. Some variants may become unused over
/// time and will then get deprecated before being removed.
///
/// The list of variants may grow. Matching against this exhaustively is not covered by interface
/// stability guarantees.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[expect(missing_docs)]
#[non_exhaustive]
pub enum TiffUnsupportedError {
    // FloatingPointPredictor(ColorType),
    // HorizontalPredictor(ColorType),
    InconsistentBitsPerSample(Vec<u8>),
    InterpretationWithBits(PhotometricInterpretation, Vec<u8>),
    UnknownInterpretation,
    UnknownCompressionMethod,
    UnsupportedCompressionMethod(CompressionMethod),
    UnsupportedPredictor(Predictor),
    UnsupportedSampleDepth(u8),
    UnsupportedSampleFormat(Vec<SampleFormat>),
    // UnsupportedColorType(ColorType),
    UnsupportedBitsPerChannel(u8),
    UnsupportedPlanarConfig(Option<PlanarConfiguration>),
    UnsupportedDataType,
    UnsupportedInterpretation(PhotometricInterpretation),
    UnsupportedJpegFeature(UnsupportedFeature),
    MisalignedTileBoundaries,
}

impl fmt::Display for TiffUnsupportedError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use self::TiffUnsupportedError::*;
        match *self {
            // FloatingPointPredictor(color_type) => write!(
            //     fmt,
            //     "Floating point predictor for {:?} is unsupported.",
            //     color_type
            // ),
            // HorizontalPredictor(color_type) => write!(
            //     fmt,
            //     "Horizontal predictor for {:?} is unsupported.",
            //     color_type
            // ),
            InconsistentBitsPerSample(ref bits_per_sample) => {
                write!(fmt, "Inconsistent bits per sample: {bits_per_sample:?}.")
            }
            InterpretationWithBits(ref photometric_interpretation, ref bits_per_sample) => write!(
                fmt,
                "{photometric_interpretation:?} with {bits_per_sample:?} bits per sample is unsupported"
            ),
            UnknownInterpretation => write!(
                fmt,
                "The image is using an unknown photometric interpretation."
            ),
            UnknownCompressionMethod => write!(fmt, "Unknown compression method."),
            UnsupportedCompressionMethod(method) => {
                write!(fmt, "Compression method {method:?} is unsupported")
            }
            UnsupportedPredictor(p) => {
                write!(fmt, "Predictor {p:?} is unsupported")
            }
            UnsupportedSampleDepth(samples) => {
                write!(fmt, "{samples} samples per pixel is unsupported.")
            }
            UnsupportedSampleFormat(ref formats) => {
                write!(fmt, "Sample format {formats:?} is unsupported.")
            }
            // UnsupportedColorType(color_type) => {
            //     write!(fmt, "Color type {:?} is unsupported", color_type)
            // }
            UnsupportedBitsPerChannel(bits) => {
                write!(fmt, "{bits} bits per channel not supported")
            }
            UnsupportedPlanarConfig(config) => {
                write!(fmt, "Unsupported planar configuration “{config:?}”.")
            }
            UnsupportedDataType => write!(fmt, "Unsupported data type."),
            UnsupportedInterpretation(interpretation) => {
                write!(
                    fmt,
                    "Unsupported photometric interpretation \"{interpretation:?}\"."
                )
            }
            UnsupportedJpegFeature(ref unsupported_feature) => {
                write!(fmt, "Unsupported JPEG feature {unsupported_feature:?}")
            }
            MisalignedTileBoundaries => write!(fmt, "Tile rows are not aligned to byte boundaries"),
        }
    }
}

/// User attempted to use the Decoder in a way that is incompatible with a specific image.
///
/// For example: attempting to read a tile from a stripped image.
#[expect(missing_docs)]
#[derive(Debug)]
pub enum UsageError {
    // InvalidChunkType(ChunkType, ChunkType),
    InvalidChunkIndex(u32),
    PredictorCompressionMismatch,
    PredictorIncompatible,
    PredictorUnavailable,
}

impl fmt::Display for UsageError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::UsageError::*;
        match *self {
            // InvalidChunkType(expected, actual) => {
            //     write!(
            //         fmt,
            //         "Requested operation is only valid for images with chunk encoding of type: {:?}, got {:?}.",
            //         expected, actual
            //     )
            // }
            InvalidChunkIndex(index) => write!(fmt, "Image chunk index ({index}) requested."),
            PredictorCompressionMismatch => write!(
                fmt,
                "The requested predictor is not compatible with the requested compression"
            ),
            PredictorIncompatible => write!(
                fmt,
                "The requested predictor is not compatible with the image's format"
            ),
            PredictorUnavailable => write!(fmt, "The requested predictor is not available"),
        }
    }
}

impl fmt::Display for TiffError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            TiffError::FormatError(ref e) => write!(fmt, "Format error: {e}"),
            TiffError::UnsupportedError(ref f) => write!(
                fmt,
                "The Decoder does not support the \
                 image format `{f}`"
            ),
            TiffError::IoError(ref e) => write!(fmt, "{}", e),
            TiffError::IntSizeError => write!(fmt, "Platform or format size limits exceeded"),
            TiffError::UsageError(ref e) => write!(fmt, "Usage error: {e}"),
        }
    }
}

impl Error for TiffError {
    fn description(&self) -> &str {
        match *self {
            TiffError::FormatError(..) => "Format error",
            TiffError::UnsupportedError(..) => "Unsupported error",
            TiffError::IoError(..) => "IO error",
            TiffError::IntSizeError => "Platform or format size limits exceeded",
            TiffError::UsageError(..) => "Invalid usage",
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        match *self {
            TiffError::IoError(ref e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for TiffError {
    fn from(err: io::Error) -> TiffError {
        TiffError::IoError(err)
    }
}

impl From<str::Utf8Error> for TiffError {
    fn from(_err: str::Utf8Error) -> TiffError {
        TiffError::FormatError(TiffFormatError::InvalidTag)
    }
}

impl From<string::FromUtf8Error> for TiffError {
    fn from(_err: string::FromUtf8Error) -> TiffError {
        TiffError::FormatError(TiffFormatError::InvalidTag)
    }
}

impl From<TiffFormatError> for TiffError {
    fn from(err: TiffFormatError) -> TiffError {
        TiffError::FormatError(err)
    }
}

impl From<TiffUnsupportedError> for TiffError {
    fn from(err: TiffUnsupportedError) -> TiffError {
        TiffError::UnsupportedError(err)
    }
}

impl From<UsageError> for TiffError {
    fn from(err: UsageError) -> TiffError {
        TiffError::UsageError(err)
    }
}

impl From<std::num::TryFromIntError> for TiffError {
    fn from(_err: std::num::TryFromIntError) -> TiffError {
        TiffError::IntSizeError
    }
}

// impl From<LzwError> for TiffError {
//     fn from(err: LzwError) -> TiffError {
//         match err {
//             LzwError::InvalidCode => TiffError::FormatError(TiffFormatError::Format(String::from(
//                 "LZW compressed data corrupted",
//             ))),
//         }
//     }
// }

/// Result of an image decoding/encoding process
pub type TiffResult<T> = Result<T, TiffError>;
