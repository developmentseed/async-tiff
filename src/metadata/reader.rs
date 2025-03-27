use std::collections::HashMap;
use std::io::Read;

use bytes::Bytes;

use crate::error::{AsyncTiffError, AsyncTiffResult};
use crate::metadata::fetch::MetadataCursor;
use crate::metadata::MetadataFetch;
use crate::reader::Endianness;
use crate::tiff::tags::{Tag, Type};
use crate::tiff::{TiffError, TiffFormatError, Value};
use crate::ImageFileDirectory;

/// Entry point to reading TIFF metadata.
///
/// This is a stateful reader because we don't know how many IFDs will be encountered.
///
/// ```notest
/// // fetch implements MetadataFetch
/// let mut metadata_reader = TiffMetadataReader::try_open(&fetch).await?;
/// let ifds = metadata_reader.read_all_ifds(&fetch).await?;
/// ```
pub struct TiffMetadataReader {
    endianness: Endianness,
    bigtiff: bool,
    next_ifd_offset: Option<u64>,
}

impl TiffMetadataReader {
    /// Open a new TIFF file, validating the magic bytes, reading the endianness, and checking for
    /// the bigtiff flag.
    ///
    /// This does not read any IFD metadata.
    pub async fn try_open<F: MetadataFetch>(fetch: &F) -> AsyncTiffResult<Self> {
        let magic_bytes = fetch.fetch(0..2).await?;

        // Should be b"II" for little endian or b"MM" for big endian
        let endianness = if magic_bytes == Bytes::from_static(b"II") {
            Endianness::LittleEndian
        } else if magic_bytes == Bytes::from_static(b"MM") {
            Endianness::BigEndian
        } else {
            return Err(AsyncTiffError::General(format!(
                "unexpected magic bytes {magic_bytes:?}"
            )));
        };

        // Set offset to 2 since we've already read magic bytes.
        let mut cursor = MetadataCursor::new(fetch, endianness).with_offset(2);

        let version = cursor.read_u16().await?;
        let bigtiff = match version {
            42 => false,
            43 => {
                // Read bytesize of offsets (in bigtiff it's alway 8 but provide a way to move to 16 some day)
                if cursor.read_u16().await? != 8 {
                    return Err(
                        TiffError::FormatError(TiffFormatError::TiffSignatureNotFound).into(),
                    );
                }
                // This constant should always be 0
                if cursor.read_u16().await? != 0 {
                    return Err(
                        TiffError::FormatError(TiffFormatError::TiffSignatureNotFound).into(),
                    );
                }
                true
            }
            _ => return Err(TiffError::FormatError(TiffFormatError::TiffSignatureInvalid).into()),
        };

        let first_ifd_location = if bigtiff {
            cursor.read_u64().await?
        } else {
            cursor.read_u32().await?.into()
        };

        Ok(Self {
            endianness,
            bigtiff,
            next_ifd_offset: Some(first_ifd_location),
        })
    }

    /// Returns the endianness of the file.
    pub fn endianness(&self) -> Endianness {
        self.endianness
    }

    /// Returns `true` if this is a bigtiff file.
    pub fn bigtiff(&self) -> bool {
        self.bigtiff
    }

    /// Returns `true` if there are more IFDs to read.
    pub fn has_next_ifd(&self) -> bool {
        self.next_ifd_offset.is_some()
    }

    /// The byte offset of the start of the next IFD.
    ///
    /// This will be `None` if all IFDs have already been read.
    pub fn next_ifd_offset(&self) -> Option<u64> {
        self.next_ifd_offset
    }

    /// Read the next IFD from the file.
    ///
    /// If there are no more IFDs, returns `None`.
    pub async fn read_next_ifd<F: MetadataFetch>(
        &mut self,
        fetch: &F,
    ) -> AsyncTiffResult<Option<ImageFileDirectory>> {
        if let Some(ifd_start) = self.next_ifd_offset {
            let ifd_reader =
                ImageFileDirectoryReader::open(fetch, ifd_start, self.bigtiff, self.endianness)
                    .await?;
            let ifd = ifd_reader.read(fetch).await?;
            let next_ifd_offset = ifd_reader.finish(fetch).await?;
            self.next_ifd_offset = next_ifd_offset;
            Ok(Some(ifd))
        } else {
            Ok(None)
        }
    }

    /// Read all IFDs from the file.
    pub async fn read_all_ifds<F: MetadataFetch>(
        &mut self,
        fetch: &F,
    ) -> AsyncTiffResult<Vec<ImageFileDirectory>> {
        let mut ifds = vec![];
        while let Some(ifd) = self.read_next_ifd(fetch).await? {
            ifds.push(ifd);
        }
        Ok(ifds)
    }
}

/// Reads the [`ImageFileDirectory`] metadata.
///
/// TIFF metadata is not necessarily contiguous in the files: IFDs are normally all stored
/// contiguously in the header, but the spec allows them to be non-contiguous or spread out through
/// the file.
///
/// Note that you must call [`finish`][ImageFileDirectoryReader::finish] to read the offset of the
/// following IFD.
pub struct ImageFileDirectoryReader {
    endianness: Endianness,
    bigtiff: bool,
    /// The byte offset of the beginning of this IFD
    ifd_start_offset: u64,
    /// The number of tags in this IFD
    tag_count: u64,
    /// The number of bytes that each IFD entry takes up.
    /// This is 12 bytes for normal TIFF and 20 bytes for BigTIFF.
    ifd_entry_byte_size: u64,
    /// The number of bytes that the value for the number of tags takes up.
    tag_count_byte_size: u64,
}

impl ImageFileDirectoryReader {
    /// Read and parse the IFD starting at the given file offset
    pub async fn open<F: MetadataFetch>(
        fetch: &F,
        ifd_start_offset: u64,
        bigtiff: bool,
        endianness: Endianness,
    ) -> AsyncTiffResult<Self> {
        let mut cursor = MetadataCursor::new_with_offset(fetch, endianness, ifd_start_offset);

        // Tag   2 bytes
        // Type  2 bytes
        // Count:
        //  - bigtiff: 8 bytes
        //  - else: 4 bytes
        // Value:
        //  - bigtiff: 8 bytes either a pointer the value itself
        //  - else: 4 bytes either a pointer the value itself
        let ifd_entry_byte_size = if bigtiff { 20 } else { 12 };
        // The size of `tag_count` that we read above
        let tag_count_byte_size = if bigtiff { 8 } else { 2 };

        let tag_count = if bigtiff {
            cursor.read_u64().await?
        } else {
            cursor.read_u16().await?.into()
        };

        Ok(Self {
            endianness,
            bigtiff,
            ifd_entry_byte_size,
            tag_count,
            tag_count_byte_size,
            ifd_start_offset,
        })
    }

    /// Manually read the tag with the specified index.
    ///
    /// Panics if the tag index is out of range of the tag count.
    ///
    /// This can be useful if you need to access tags at a low level. You'll need to call
    /// [`ImageFileDirectory::from_tags`] on the resulting collection of tags.
    pub async fn read_tag<F: MetadataFetch>(
        &self,
        fetch: &F,
        tag_idx: u64,
    ) -> AsyncTiffResult<(Tag, Value)> {
        assert!(tag_idx < self.tag_count);
        let tag_offset =
            self.ifd_start_offset + self.tag_count_byte_size + (self.ifd_entry_byte_size * tag_idx);
        let (tag_name, tag_value) =
            read_tag(fetch, tag_offset, self.endianness, self.bigtiff).await?;
        Ok((tag_name, tag_value))
    }

    /// Read all tags out of this IFD.
    ///
    /// Keep in mind that you'll still need to call [`finish`][Self::finish] to get the byte offset
    /// of the next IFD.
    pub async fn read<F: MetadataFetch>(&self, fetch: &F) -> AsyncTiffResult<ImageFileDirectory> {
        let mut tags = HashMap::with_capacity(self.tag_count as usize);
        for tag_idx in 0..self.tag_count {
            let (tag, value) = self.read_tag(fetch, tag_idx).await?;
            tags.insert(tag, value);
        }
        ImageFileDirectory::from_tags(tags)
    }

    /// Finish this reader, reading the byte offset of the next IFD
    pub async fn finish<F: MetadataFetch>(self, fetch: &F) -> AsyncTiffResult<Option<u64>> {
        // The byte offset for reading the next ifd
        let next_ifd_byte_offset = self.ifd_start_offset
            + self.tag_count_byte_size
            + (self.ifd_entry_byte_size * self.tag_count);
        let mut cursor =
            MetadataCursor::new_with_offset(fetch, self.endianness, next_ifd_byte_offset);

        let next_ifd_offset = if self.bigtiff {
            cursor.read_u64().await?
        } else {
            cursor.read_u32().await?.into()
        };

        // If the ifd_offset is 0, no more IFDs
        if next_ifd_offset == 0 {
            Ok(None)
        } else {
            Ok(Some(next_ifd_offset))
        }
    }
}

/// Read a single tag from the cursor
async fn read_tag<F: MetadataFetch>(
    fetch: &F,
    tag_offset: u64,
    endianness: Endianness,
    bigtiff: bool,
) -> AsyncTiffResult<(Tag, Value)> {
    let mut cursor = MetadataCursor::new_with_offset(fetch, endianness, tag_offset);

    let tag_name = Tag::from_u16_exhaustive(cursor.read_u16().await?);

    let tag_type_code = cursor.read_u16().await?;
    let tag_type = Type::from_u16(tag_type_code).expect(
        "Unknown tag type {tag_type_code}. TODO: we should skip entries with unknown tag types.",
    );
    let count = if bigtiff {
        cursor.read_u64().await?
    } else {
        cursor.read_u32().await?.into()
    };

    let tag_value = read_tag_value(&mut cursor, tag_type, count, bigtiff).await?;

    Ok((tag_name, tag_value))
}

/// Read a tag's value from the cursor
///
/// NOTE: this does not maintain cursor state
// This is derived from the upstream tiff crate:
// https://github.com/image-rs/image-tiff/blob/6dc7a266d30291db1e706c8133357931f9e2a053/src/decoder/ifd.rs#L369-L639
async fn read_tag_value<F: MetadataFetch>(
    cursor: &mut MetadataCursor<'_, F>,
    tag_type: Type,
    count: u64,
    bigtiff: bool,
) -> AsyncTiffResult<Value> {
    // Case 1: there are no values so we can return immediately.
    if count == 0 {
        return Ok(Value::List(vec![]));
    }

    let tag_size = match tag_type {
        Type::BYTE | Type::SBYTE | Type::ASCII | Type::UNDEFINED => 1,
        Type::SHORT | Type::SSHORT => 2,
        Type::LONG | Type::SLONG | Type::FLOAT | Type::IFD => 4,
        Type::LONG8
        | Type::SLONG8
        | Type::DOUBLE
        | Type::RATIONAL
        | Type::SRATIONAL
        | Type::IFD8 => 8,
    };

    let value_byte_length = count.checked_mul(tag_size).unwrap();

    // Case 2: there is one value.
    if count == 1 {
        // 2a: the value is 5-8 bytes and we're in BigTiff mode.
        if bigtiff && value_byte_length > 4 && value_byte_length <= 8 {
            let mut data = cursor.read(value_byte_length).await?;

            return Ok(match tag_type {
                Type::LONG8 => Value::UnsignedBig(data.read_u64()?),
                Type::SLONG8 => Value::SignedBig(data.read_i64()?),
                Type::DOUBLE => Value::Double(data.read_f64()?),
                Type::RATIONAL => Value::Rational(data.read_u32()?, data.read_u32()?),
                Type::SRATIONAL => Value::SRational(data.read_i32()?, data.read_i32()?),
                Type::IFD8 => Value::IfdBig(data.read_u64()?),
                Type::BYTE
                | Type::SBYTE
                | Type::ASCII
                | Type::UNDEFINED
                | Type::SHORT
                | Type::SSHORT
                | Type::LONG
                | Type::SLONG
                | Type::FLOAT
                | Type::IFD => unreachable!(),
            });
        }

        // NOTE: we should only be reading value_byte_length when it's 4 bytes or fewer. Right now
        // we're reading even if it's 8 bytes, but then only using the first 4 bytes of this
        // buffer.
        let mut data = cursor.read(value_byte_length).await?;

        // 2b: the value is at most 4 bytes or doesn't fit in the offset field.
        return Ok(match tag_type {
            Type::BYTE | Type::UNDEFINED => Value::Byte(data.read_u8()?),
            Type::SBYTE => Value::Signed(data.read_i8()? as i32),
            Type::SHORT => Value::Short(data.read_u16()?),
            Type::SSHORT => Value::Signed(data.read_i16()? as i32),
            Type::LONG => Value::Unsigned(data.read_u32()?),
            Type::SLONG => Value::Signed(data.read_i32()?),
            Type::FLOAT => Value::Float(data.read_f32()?),
            Type::ASCII => {
                if data.as_ref()[0] == 0 {
                    Value::Ascii("".to_string())
                } else {
                    panic!("Invalid tag");
                    // return Err(TiffError::FormatError(TiffFormatError::InvalidTag));
                }
            }
            Type::LONG8 => {
                let offset = data.read_u32()?;
                cursor.seek(offset as _);
                Value::UnsignedBig(cursor.read_u64().await?)
            }
            Type::SLONG8 => {
                let offset = data.read_u32()?;
                cursor.seek(offset as _);
                Value::SignedBig(cursor.read_i64().await?)
            }
            Type::DOUBLE => {
                let offset = data.read_u32()?;
                cursor.seek(offset as _);
                Value::Double(cursor.read_f64().await?)
            }
            Type::RATIONAL => {
                let offset = data.read_u32()?;
                cursor.seek(offset as _);
                let numerator = cursor.read_u32().await?;
                let denominator = cursor.read_u32().await?;
                Value::Rational(numerator, denominator)
            }
            Type::SRATIONAL => {
                let offset = data.read_u32()?;
                cursor.seek(offset as _);
                let numerator = cursor.read_i32().await?;
                let denominator = cursor.read_i32().await?;
                Value::SRational(numerator, denominator)
            }
            Type::IFD => Value::Ifd(data.read_u32()?),
            Type::IFD8 => {
                let offset = data.read_u32()?;
                cursor.seek(offset as _);
                Value::IfdBig(cursor.read_u64().await?)
            }
        });
    }

    // Case 3: There is more than one value, but it fits in the offset field.
    if value_byte_length <= 4 || bigtiff && value_byte_length <= 8 {
        let mut data = cursor.read(value_byte_length).await?;
        if bigtiff {
            cursor.advance(8 - value_byte_length);
        } else {
            cursor.advance(4 - value_byte_length);
        }

        match tag_type {
            Type::BYTE | Type::UNDEFINED => {
                return {
                    Ok(Value::List(
                        (0..count)
                            .map(|_| Value::Byte(data.read_u8().unwrap()))
                            .collect(),
                    ))
                };
            }
            Type::SBYTE => {
                return {
                    Ok(Value::List(
                        (0..count)
                            .map(|_| Value::Signed(data.read_i8().unwrap() as i32))
                            .collect(),
                    ))
                }
            }
            Type::ASCII => {
                let mut buf = vec![0; count as usize];
                data.read_exact(&mut buf)?;
                if buf.is_ascii() && buf.ends_with(&[0]) {
                    let v = std::str::from_utf8(&buf)
                        .map_err(|err| AsyncTiffError::General(err.to_string()))?;
                    let v = v.trim_matches(char::from(0));
                    return Ok(Value::Ascii(v.into()));
                } else {
                    panic!("Invalid tag");
                    // return Err(TiffError::FormatError(TiffFormatError::InvalidTag));
                }
            }
            Type::SHORT => {
                let mut v = Vec::new();
                for _ in 0..count {
                    v.push(Value::Short(data.read_u16()?));
                }
                return Ok(Value::List(v));
            }
            Type::SSHORT => {
                let mut v = Vec::new();
                for _ in 0..count {
                    v.push(Value::Signed(i32::from(data.read_i16()?)));
                }
                return Ok(Value::List(v));
            }
            Type::LONG => {
                let mut v = Vec::new();
                for _ in 0..count {
                    v.push(Value::Unsigned(data.read_u32()?));
                }
                return Ok(Value::List(v));
            }
            Type::SLONG => {
                let mut v = Vec::new();
                for _ in 0..count {
                    v.push(Value::Signed(data.read_i32()?));
                }
                return Ok(Value::List(v));
            }
            Type::FLOAT => {
                let mut v = Vec::new();
                for _ in 0..count {
                    v.push(Value::Float(data.read_f32()?));
                }
                return Ok(Value::List(v));
            }
            Type::IFD => {
                let mut v = Vec::new();
                for _ in 0..count {
                    v.push(Value::Ifd(data.read_u32()?));
                }
                return Ok(Value::List(v));
            }
            Type::LONG8
            | Type::SLONG8
            | Type::RATIONAL
            | Type::SRATIONAL
            | Type::DOUBLE
            | Type::IFD8 => {
                unreachable!()
            }
        }
    }

    // Seek cursor
    let offset = if bigtiff {
        cursor.read_u64().await?
    } else {
        cursor.read_u32().await?.into()
    };
    cursor.seek(offset);

    // Case 4: there is more than one value, and it doesn't fit in the offset field.
    match tag_type {
        // TODO check if this could give wrong results
        // at a different endianess of file/computer.
        Type::BYTE | Type::UNDEFINED => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Byte(cursor.read_u8().await?))
            }
            Ok(Value::List(v))
        }
        Type::SBYTE => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Signed(cursor.read_i8().await? as i32))
            }
            Ok(Value::List(v))
        }
        Type::SHORT => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Short(cursor.read_u16().await?))
            }
            Ok(Value::List(v))
        }
        Type::SSHORT => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Signed(cursor.read_i16().await? as i32))
            }
            Ok(Value::List(v))
        }
        Type::LONG => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Unsigned(cursor.read_u32().await?))
            }
            Ok(Value::List(v))
        }
        Type::SLONG => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Signed(cursor.read_i32().await?))
            }
            Ok(Value::List(v))
        }
        Type::FLOAT => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Float(cursor.read_f32().await?))
            }
            Ok(Value::List(v))
        }
        Type::DOUBLE => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Double(cursor.read_f64().await?))
            }
            Ok(Value::List(v))
        }
        Type::RATIONAL => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Rational(
                    cursor.read_u32().await?,
                    cursor.read_u32().await?,
                ))
            }
            Ok(Value::List(v))
        }
        Type::SRATIONAL => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::SRational(
                    cursor.read_i32().await?,
                    cursor.read_i32().await?,
                ))
            }
            Ok(Value::List(v))
        }
        Type::LONG8 => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::UnsignedBig(cursor.read_u64().await?))
            }
            Ok(Value::List(v))
        }
        Type::SLONG8 => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::SignedBig(cursor.read_i64().await?))
            }
            Ok(Value::List(v))
        }
        Type::IFD => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Ifd(cursor.read_u32().await?))
            }
            Ok(Value::List(v))
        }
        Type::IFD8 => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::IfdBig(cursor.read_u64().await?))
            }
            Ok(Value::List(v))
        }
        Type::ASCII => {
            let mut out = vec![0; count as _];
            let mut buf = cursor.read(count).await?;
            buf.read_exact(&mut out)?;

            // Strings may be null-terminated, so we trim anything downstream of the null byte
            if let Some(first) = out.iter().position(|&b| b == 0) {
                out.truncate(first);
            }
            Ok(Value::Ascii(
                String::from_utf8(out).map_err(|err| AsyncTiffError::General(err.to_string()))?,
            ))
        }
    }
}
