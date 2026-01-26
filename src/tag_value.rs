use self::TagValue::{
    Ascii, Byte, Double, Float, Ifd, IfdBig, List, Rational, RationalBig, SRational, SRationalBig,
    Short, Signed, SignedBig, SignedByte, SignedShort, Unsigned, UnsignedBig,
};
use crate::error::{TiffError, TiffFormatError, TiffResult};
// use super::error::{TiffError, TiffFormatError, TiffResult};

/// A dynamically-typed value parsed from a TIFF IFD entry.
///
/// Each variant corresponds to one of the TIFF data types. Multi-value entries are represented
/// as [`TagValue::List`].
///
/// Conversion methods like [`into_u16`](TagValue::into_u16) and
/// [`into_f64_vec`](TagValue::into_f64_vec) handle widening casts and return an error on
/// type mismatches.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum TagValue {
    /// 8-bit unsigned integer (TIFF type BYTE).
    Byte(u8),
    /// 16-bit unsigned integer (TIFF type SHORT).
    Short(u16),
    /// 8-bit signed integer (TIFF type SBYTE).
    SignedByte(i8),
    /// 16-bit signed integer (TIFF type SSHORT).
    SignedShort(i16),
    /// 32-bit signed integer (TIFF type SLONG).
    Signed(i32),
    /// 64-bit signed integer (BigTIFF type SLONG8).
    SignedBig(i64),
    /// 32-bit unsigned integer (TIFF type LONG).
    Unsigned(u32),
    /// 64-bit unsigned integer (BigTIFF type LONG8).
    UnsignedBig(u64),
    /// 32-bit IEEE floating point (TIFF type FLOAT).
    Float(f32),
    /// 64-bit IEEE floating point (TIFF type DOUBLE).
    Double(f64),
    /// A sequence of values from a multi-count IFD entry.
    List(Vec<TagValue>),
    /// Unsigned rational: numerator and denominator (TIFF type RATIONAL).
    Rational(u32, u32),
    /// Unsigned rational with 64-bit components (BigTIFF).
    RationalBig(u64, u64),
    /// Signed rational: numerator and denominator (TIFF type SRATIONAL).
    SRational(i32, i32),
    /// Signed rational with 64-bit components (BigTIFF).
    SRationalBig(i64, i64),
    /// ASCII string parsed from TIFF type ASCII
    Ascii(String),
    /// 32-bit IFD offset (TIFF type IFD).
    Ifd(u32),
    /// 64-bit IFD offset (BigTIFF type IFD8).
    IfdBig(u64),
}

impl TagValue {
    /// Convert this TagValue into a u8, returning an error if the type is incompatible.
    pub fn into_u8(self) -> TiffResult<u8> {
        match self {
            Byte(val) => Ok(val),
            val => Err(TiffError::FormatError(TiffFormatError::ByteExpected(val))),
        }
    }

    /// Convert this TagValue into an i8, returning an error if the type is incompatible.
    pub fn into_i8(self) -> TiffResult<i8> {
        match self {
            SignedByte(val) => Ok(val),
            val => Err(TiffError::FormatError(TiffFormatError::SignedByteExpected(
                val,
            ))),
        }
    }

    /// Convert this TagValue into a u16, returning an error if the type is incompatible.
    pub fn into_u16(self) -> TiffResult<u16> {
        match self {
            Byte(val) => Ok(val.into()),
            Short(val) => Ok(val),
            Unsigned(val) => Ok(u16::try_from(val)?),
            UnsignedBig(val) => Ok(u16::try_from(val)?),
            val => Err(TiffError::FormatError(TiffFormatError::ShortExpected(val))),
        }
    }

    /// Convert this TagValue into an i16, returning an error if the type is incompatible.
    pub fn into_i16(self) -> TiffResult<i16> {
        match self {
            SignedByte(val) => Ok(val.into()),
            SignedShort(val) => Ok(val),
            Signed(val) => Ok(i16::try_from(val)?),
            SignedBig(val) => Ok(i16::try_from(val)?),
            val => Err(TiffError::FormatError(
                TiffFormatError::SignedShortExpected(val),
            )),
        }
    }

    /// Convert this TagValue into a u32, returning an error if the type is incompatible.
    pub fn into_u32(self) -> TiffResult<u32> {
        match self {
            Byte(val) => Ok(val.into()),
            Short(val) => Ok(val.into()),
            Unsigned(val) => Ok(val),
            UnsignedBig(val) => Ok(u32::try_from(val)?),
            Ifd(val) => Ok(val),
            IfdBig(val) => Ok(u32::try_from(val)?),
            val => Err(TiffError::FormatError(
                TiffFormatError::UnsignedIntegerExpected(val),
            )),
        }
    }

    /// Convert this TagValue into an i32, returning an error if the type is incompatible.
    pub fn into_i32(self) -> TiffResult<i32> {
        match self {
            SignedByte(val) => Ok(val.into()),
            SignedShort(val) => Ok(val.into()),
            Signed(val) => Ok(val),
            SignedBig(val) => Ok(i32::try_from(val)?),
            val => Err(TiffError::FormatError(
                TiffFormatError::SignedIntegerExpected(val),
            )),
        }
    }

    /// Convert this TagValue into a u64, returning an error if the type is incompatible.
    pub fn into_u64(self) -> TiffResult<u64> {
        match self {
            Byte(val) => Ok(val.into()),
            Short(val) => Ok(val.into()),
            Unsigned(val) => Ok(val.into()),
            UnsignedBig(val) => Ok(val),
            Ifd(val) => Ok(val.into()),
            IfdBig(val) => Ok(val),
            val => Err(TiffError::FormatError(
                TiffFormatError::UnsignedIntegerExpected(val),
            )),
        }
    }

    /// Convert this TagValue into an i64, returning an error if the type is incompatible.
    pub fn into_i64(self) -> TiffResult<i64> {
        match self {
            SignedByte(val) => Ok(val.into()),
            SignedShort(val) => Ok(val.into()),
            Signed(val) => Ok(val.into()),
            SignedBig(val) => Ok(val),
            val => Err(TiffError::FormatError(
                TiffFormatError::SignedIntegerExpected(val),
            )),
        }
    }

    /// Convert this TagValue into a f32, returning an error if the type is incompatible.
    pub fn into_f32(self) -> TiffResult<f32> {
        match self {
            Float(val) => Ok(val),
            val => Err(TiffError::FormatError(
                TiffFormatError::SignedIntegerExpected(val),
            )),
        }
    }

    /// Convert this TagValue into a f64, returning an error if the type is incompatible.
    pub fn into_f64(self) -> TiffResult<f64> {
        match self {
            Double(val) => Ok(val),
            val => Err(TiffError::FormatError(
                TiffFormatError::SignedIntegerExpected(val),
            )),
        }
    }

    /// Convert this TagValue into a String, returning an error if the type is incompatible.
    pub fn into_string(self) -> TiffResult<String> {
        match self {
            Ascii(val) => Ok(val),
            val => Err(TiffError::FormatError(
                TiffFormatError::SignedIntegerExpected(val),
            )),
        }
    }

    /// Convert this TagValue into a `Vec<u32>`, returning an error if the type is incompatible.
    pub fn into_u32_vec(self) -> TiffResult<Vec<u32>> {
        match self {
            List(vec) => {
                let mut new_vec = Vec::with_capacity(vec.len());
                for v in vec {
                    new_vec.push(v.into_u32()?)
                }
                Ok(new_vec)
            }
            Byte(val) => Ok(vec![val.into()]),
            Short(val) => Ok(vec![val.into()]),
            Unsigned(val) => Ok(vec![val]),
            UnsignedBig(val) => Ok(vec![u32::try_from(val)?]),
            Rational(numerator, denominator) => Ok(vec![numerator, denominator]),
            RationalBig(numerator, denominator) => {
                Ok(vec![u32::try_from(numerator)?, u32::try_from(denominator)?])
            }
            Ifd(val) => Ok(vec![val]),
            IfdBig(val) => Ok(vec![u32::try_from(val)?]),
            Ascii(val) => Ok(val.chars().map(u32::from).collect()),
            val => Err(TiffError::FormatError(
                TiffFormatError::UnsignedIntegerExpected(val),
            )),
        }
    }

    /// Convert this TagValue into a `Vec<u8>`, returning an error if the type is incompatible.
    pub fn into_u8_vec(self) -> TiffResult<Vec<u8>> {
        match self {
            List(vec) => {
                let mut new_vec = Vec::with_capacity(vec.len());
                for v in vec {
                    new_vec.push(v.into_u8()?)
                }
                Ok(new_vec)
            }
            Byte(val) => Ok(vec![val]),
            val => Err(TiffError::FormatError(TiffFormatError::ByteExpected(val))),
        }
    }

    /// Convert this TagValue into a `Vec<u16>`, returning an error if the type is incompatible.
    pub fn into_u16_vec(self) -> TiffResult<Vec<u16>> {
        match self {
            List(vec) => {
                let mut new_vec = Vec::with_capacity(vec.len());
                for v in vec {
                    new_vec.push(v.into_u16()?)
                }
                Ok(new_vec)
            }
            Byte(val) => Ok(vec![val.into()]),
            Short(val) => Ok(vec![val]),
            val => Err(TiffError::FormatError(TiffFormatError::ShortExpected(val))),
        }
    }

    /// Convert this TagValue into a `Vec<i32>`, returning an error if the type is incompatible.
    pub fn into_i32_vec(self) -> TiffResult<Vec<i32>> {
        match self {
            List(vec) => {
                let mut new_vec = Vec::with_capacity(vec.len());
                for v in vec {
                    match v {
                        SRational(numerator, denominator) => {
                            new_vec.push(numerator);
                            new_vec.push(denominator);
                        }
                        SRationalBig(numerator, denominator) => {
                            new_vec.push(i32::try_from(numerator)?);
                            new_vec.push(i32::try_from(denominator)?);
                        }
                        _ => new_vec.push(v.into_i32()?),
                    }
                }
                Ok(new_vec)
            }
            SignedByte(val) => Ok(vec![val.into()]),
            SignedShort(val) => Ok(vec![val.into()]),
            Signed(val) => Ok(vec![val]),
            SignedBig(val) => Ok(vec![i32::try_from(val)?]),
            SRational(numerator, denominator) => Ok(vec![numerator, denominator]),
            SRationalBig(numerator, denominator) => {
                Ok(vec![i32::try_from(numerator)?, i32::try_from(denominator)?])
            }
            val => Err(TiffError::FormatError(
                TiffFormatError::SignedIntegerExpected(val),
            )),
        }
    }

    /// Convert this TagValue into a `Vec<f32>`, returning an error if the type is incompatible.
    pub fn into_f32_vec(self) -> TiffResult<Vec<f32>> {
        match self {
            List(vec) => {
                let mut new_vec = Vec::with_capacity(vec.len());
                for v in vec {
                    new_vec.push(v.into_f32()?)
                }
                Ok(new_vec)
            }
            Float(val) => Ok(vec![val]),
            val => Err(TiffError::FormatError(
                TiffFormatError::UnsignedIntegerExpected(val),
            )),
        }
    }

    /// Convert this TagValue into a `Vec<f64>`, returning an error if the type is incompatible.
    pub fn into_f64_vec(self) -> TiffResult<Vec<f64>> {
        match self {
            List(vec) => {
                let mut new_vec = Vec::with_capacity(vec.len());
                for v in vec {
                    new_vec.push(v.into_f64()?)
                }
                Ok(new_vec)
            }
            Double(val) => Ok(vec![val]),
            val => Err(TiffError::FormatError(
                TiffFormatError::UnsignedIntegerExpected(val),
            )),
        }
    }

    /// Convert this TagValue into a `Vec<u64>`, returning an error if the type is incompatible.
    pub fn into_u64_vec(self) -> TiffResult<Vec<u64>> {
        match self {
            List(vec) => {
                let mut new_vec = Vec::with_capacity(vec.len());
                for v in vec {
                    new_vec.push(v.into_u64()?)
                }
                Ok(new_vec)
            }
            Byte(val) => Ok(vec![val.into()]),
            Short(val) => Ok(vec![val.into()]),
            Unsigned(val) => Ok(vec![val.into()]),
            UnsignedBig(val) => Ok(vec![val]),
            Rational(numerator, denominator) => Ok(vec![numerator.into(), denominator.into()]),
            RationalBig(numerator, denominator) => Ok(vec![numerator, denominator]),
            Ifd(val) => Ok(vec![val.into()]),
            IfdBig(val) => Ok(vec![val]),
            Ascii(val) => Ok(val.chars().map(u32::from).map(u64::from).collect()),
            val => Err(TiffError::FormatError(
                TiffFormatError::UnsignedIntegerExpected(val),
            )),
        }
    }

    /// Convert this TagValue into a `Vec<i64>`, returning an error if the type is incompatible.
    pub fn into_i64_vec(self) -> TiffResult<Vec<i64>> {
        match self {
            List(vec) => {
                let mut new_vec = Vec::with_capacity(vec.len());
                for v in vec {
                    match v {
                        SRational(numerator, denominator) => {
                            new_vec.push(numerator.into());
                            new_vec.push(denominator.into());
                        }
                        SRationalBig(numerator, denominator) => {
                            new_vec.push(numerator);
                            new_vec.push(denominator);
                        }
                        _ => new_vec.push(v.into_i64()?),
                    }
                }
                Ok(new_vec)
            }
            SignedByte(val) => Ok(vec![val.into()]),
            SignedShort(val) => Ok(vec![val.into()]),
            Signed(val) => Ok(vec![val.into()]),
            SignedBig(val) => Ok(vec![val]),
            SRational(numerator, denominator) => Ok(vec![numerator.into(), denominator.into()]),
            SRationalBig(numerator, denominator) => Ok(vec![numerator, denominator]),
            val => Err(TiffError::FormatError(
                TiffFormatError::SignedIntegerExpected(val),
            )),
        }
    }
}
