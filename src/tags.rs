//! TIFF tag definitions and enum types.

#![allow(clippy::no_effect)]
#![allow(missing_docs)]
#![allow(clippy::upper_case_acronyms)]

macro_rules! tags {
    {
        // Permit arbitrary meta items, which include documentation.
        $( #[$enum_attr:meta] )*
        $vis:vis enum $name:ident($ty:tt) $(unknown($unknown_doc:literal))* {
            // Each of the `Name = Val,` permitting documentation.
            $($(#[$ident_attr:meta])* $tag:ident = $val:expr,)*
        }
    } => {
        $( #[$enum_attr] )*
        #[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
        #[non_exhaustive]
        $vis enum $name {
            $($(#[$ident_attr])* $tag,)*
            $(
                #[doc = $unknown_doc]
                Unknown($ty),
            )*
        }

        impl $name {
            #[inline(always)]
            fn __from_inner_type(n: $ty) -> Result<Self, $ty> {
                match n {
                    $( $val => Ok($name::$tag), )*
                    n => Err(n),
                }
            }

            #[inline(always)]
            fn __to_inner_type(&self) -> $ty {
                match *self {
                    $( $name::$tag => $val, )*
                    $( $name::Unknown(n) => { $unknown_doc; n }, )*
                }
            }
        }

        tags!($name, $ty, $($unknown_doc)*);
    };
    // For u16 tags, provide direct inherent primitive conversion methods.
    ($name:tt, u16, $($unknown_doc:literal)*) => {
        impl $name {
            /// Construct from a u16 value, returning `None` if the value is not a known tag.
            #[inline(always)]
            pub fn from_u16(val: u16) -> Option<Self> {
                Self::__from_inner_type(val).ok()
            }

            $(
            /// Construct from a u16 value, storing `Unknown` if the value is not a known tag.
            #[inline(always)]
            pub fn from_u16_exhaustive(val: u16) -> Self {
                $unknown_doc;
                Self::__from_inner_type(val).unwrap_or_else(|_| $name::Unknown(val))
            }
            )*

            /// Convert to a u16 value.
            #[inline(always)]
            pub fn to_u16(self) -> u16 {
                Self::__to_inner_type(&self)
            }
        }
    };
    // For other tag types, do nothing for now. With concat_idents one could
    // provide inherent conversion methods for all types.
    ($name:tt, $ty:tt, $($unknown_doc:literal)*) => {};
}

// Note: These tags appear in the order they are mentioned in the TIFF reference
tags! {
/// Tag identifiers.
pub enum Tag(u16) unknown("A private or extension tag") {
    // Baseline tags:
    Artist = 315,
    // grayscale images PhotometricInterpretation 1 or 3
    BitsPerSample = 258,
    CellLength = 265,
    CellWidth = 264,
    // palette-color images (PhotometricInterpretation 3)
    ColorMap = 320,
    Compression = 259,
    Copyright = 33_432,
    DateTime = 306,
    ExtraSamples = 338,
    FillOrder = 266,
    FreeByteCounts = 289,
    FreeOffsets = 288,
    GrayResponseCurve = 291,
    GrayResponseUnit = 290,
    HostComputer = 316,
    ImageDescription = 270,
    ImageLength = 257,
    ImageWidth = 256,
    Make = 271,
    MaxSampleValue = 281,
    MinSampleValue = 280,
    Model = 272,
    NewSubfileType = 254,
    Orientation = 274,
    PhotometricInterpretation = 262,
    PlanarConfiguration = 284,
    ResolutionUnit = 296,
    RowsPerStrip = 278,
    SamplesPerPixel = 277,
    Software = 305,
    StripByteCounts = 279,
    StripOffsets = 273,
    SubfileType = 255,
    Threshholding = 263,
    XResolution = 282,
    YResolution = 283,
    Predictor = 317,
    TileWidth = 322,
    TileLength = 323,
    TileOffsets = 324,
    TileByteCounts = 325,
    // Data Sample Format
    SampleFormat = 339,
    SMinSampleValue = 340,
    SMaxSampleValue = 341,
    // JPEG
    JPEGTables = 347,
    // GeoTIFF
    ModelPixelScale = 33550, // (SoftDesk)
    ModelTransformation = 34264, // (JPL Carto Group)
    ModelTiepoint = 33922, // (Intergraph)
    GeoKeyDirectory = 34735, // (SPOT)
    GeoDoubleParams = 34736, // (SPOT)
    GeoAsciiParams = 34737, // (SPOT)
    GdalNodata = 42113, // Contains areas with missing data
    GdalMetadata = 42112, // XML metadata string
}
}

tags! {
/// The type of an IFD entry (a 2 byte field).
pub enum Type(u16) {
    /// 8-bit unsigned integer
    BYTE = 1,
    /// 8-bit byte that contains a 7-bit ASCII code; the last byte must be zero
    ASCII = 2,
    /// 16-bit unsigned integer
    SHORT = 3,
    /// 32-bit unsigned integer
    LONG = 4,
    /// Fraction stored as two 32-bit unsigned integers
    RATIONAL = 5,
    /// 8-bit signed integer
    SBYTE = 6,
    /// 8-bit byte that may contain anything, depending on the field
    UNDEFINED = 7,
    /// 16-bit signed integer
    SSHORT = 8,
    /// 32-bit signed integer
    SLONG = 9,
    /// Fraction stored as two 32-bit signed integers
    SRATIONAL = 10,
    /// 32-bit IEEE floating point
    FLOAT = 11,
    /// 64-bit IEEE floating point
    DOUBLE = 12,
    /// 32-bit unsigned integer (offset)
    IFD = 13,
    /// BigTIFF 64-bit unsigned integer
    LONG8 = 16,
    /// BigTIFF 64-bit signed integer
    SLONG8 = 17,
    /// BigTIFF 64-bit unsigned integer (offset)
    IFD8 = 18,
}
}

tags! {
/// Known compression methods.
///
/// See [TIFF compression tags](https://www.awaresystems.be/imaging/tiff/tifftags/compression.html)
/// for reference.
pub enum CompressionMethod(u16) unknown("A custom compression method") {
    None = 1,
    Huffman = 2,
    Fax3 = 3,
    Fax4 = 4,
    LZW = 5,
    JPEG = 6,
    // "Extended JPEG" or "new JPEG" style
    ModernJPEG = 7,
    Deflate = 8,
    OldDeflate = 0x80B2,
    PackBits = 0x8005,
    JPEG2k = 34712,

    // Self-assigned by libtiff
    ZSTD = 0xC350,
}
}

tags! {
/// The color space of the image data.
pub enum PhotometricInterpretation(u16) {
    WhiteIsZero = 0,
    BlackIsZero = 1,
    RGB = 2,
    RGBPalette = 3,
    TransparencyMask = 4,
    CMYK = 5,
    YCbCr = 6,
    CIELab = 8,
}
}

tags! {
/// How pixel components are stored: contiguously (chunky) or in separate planes (planar).
pub enum PlanarConfiguration(u16) {
    Chunky = 1,
    Planar = 2,
}
}

tags! {
/// A mathematical operator applied to image data before compression to improve compression ratios.
pub enum Predictor(u16) {
    /// No changes were made to the data
    None = 1,
    /// The images' rows were processed to contain the difference of each pixel from the previous one.
    ///
    /// This means that instead of having in order `[r1, g1. b1, r2, g2 ...]` you will find
    /// `[r1, g1, b1, r2-r1, g2-g1, b2-b1, r3-r2, g3-g2, ...]`
    Horizontal = 2,
    /// Not currently supported
    FloatingPoint = 3,
}
}

tags! {
/// The unit of pixel resolution.
pub enum ResolutionUnit(u16) {
    None = 1,
    Inch = 2,
    Centimeter = 3,
}
}

tags! {
/// The format of sample values in each pixel (unsigned int, signed int, or floating point).
pub enum SampleFormat(u16) unknown("An unknown extension sample format") {
    Uint = 1,
    Int = 2,
    Float = 3,
    Void = 4,
}
}
