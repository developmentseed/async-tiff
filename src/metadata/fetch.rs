use std::ops::Range;

use bytes::Bytes;
use futures::future::BoxFuture;
use futures::FutureExt;

use crate::error::AsyncTiffResult;
use crate::reader::{AsyncFileReader, EndianAwareReader, Endianness};
use crate::tiff::Value;

#[cfg(target_endian = "little")]
const MACHINE_ENDIANNESS: Endianness = Endianness::LittleEndian;

#[cfg(target_endian = "big")]
const MACHINE_ENDIANNESS: Endianness = Endianness::BigEndian;

/// A data source that can be used with [`TiffMetadataReader`] and [`ImageFileDirectoryReader`] to
/// load [`ImageFileDirectory`]s.
///
/// Note that implementation is provided for [`AsyncFileReader`].
pub trait MetadataFetch {
    /// Return a future that fetches the specified range of bytes asynchronously
    ///
    /// Note the returned type is a boxed future, often created by
    /// [futures::FutureExt::boxed]. See the trait documentation for an example.
    fn fetch(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>>;
}

impl<T: AsyncFileReader> MetadataFetch for T {
    fn fetch(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        self.get_bytes(range)
    }
}

/// Buffering for the first `N` bytes of a file.
///
/// This is designed so that the async requests made by the underlying tag reader get intercepted
/// here and served from the existing buffer when possible.
#[derive(Debug)]
pub struct PrefetchBuffer<F: MetadataFetch> {
    fetch: F,
    buffer: Bytes,
}

impl<F: MetadataFetch> PrefetchBuffer<F> {
    /// Construct a new PrefetchBuffer, catching the first `prefetch` bytes of the file.
    pub async fn new(fetch: F, prefetch: u64) -> AsyncTiffResult<Self> {
        let buffer = fetch.fetch(0..prefetch).await?;
        Ok(Self { fetch, buffer })
    }
}

impl<F: MetadataFetch> MetadataFetch for PrefetchBuffer<F> {
    fn fetch(&self, range: Range<u64>) -> BoxFuture<'_, AsyncTiffResult<Bytes>> {
        if range.start < self.buffer.len() as _ {
            if range.end < self.buffer.len() as _ {
                let usize_range = range.start as usize..range.end as usize;
                let result = self.buffer.slice(usize_range);
                async { Ok(result) }.boxed()
            } else {
                // TODO: reuse partial internal buffer
                self.fetch.fetch(range)
            }
        } else {
            self.fetch.fetch(range)
        }
    }
}

pub(crate) struct MetadataCursor<'a, F: MetadataFetch> {
    fetch: &'a F,
    offset: u64,
    endianness: Endianness,
}

impl<'a, F: MetadataFetch> MetadataCursor<'a, F> {
    pub fn new(fetch: &'a F, endianness: Endianness) -> Self {
        Self {
            fetch,
            offset: 0,
            endianness,
        }
    }

    pub fn new_with_offset(fetch: &'a F, endianness: Endianness, offset: u64) -> Self {
        Self {
            fetch,
            offset,
            endianness,
        }
    }

    pub fn with_offset(mut self, offset: u64) -> Self {
        self.offset = offset;
        self
    }

    pub fn seek(&mut self, offset: u64) {
        self.offset = offset;
    }

    /// Advance cursor position by a set amount
    pub(crate) fn advance(&mut self, amount: u64) {
        self.offset += amount;
    }

    /// Read the given number of bytes, advancing the internal cursor state by the same amount.
    pub(crate) async fn read(&mut self, length: u64) -> AsyncTiffResult<EndianAwareReader> {
        let range = self.offset as _..(self.offset + length) as _;
        self.offset += length;
        let bytes = self.fetch.fetch(range).await?;
        Ok(EndianAwareReader::new(bytes, self.endianness))
    }

    /// Read `n` u8s from the cursor, advancing the internal state by `n` bytes.
    pub(crate) async fn read_u8_n(&mut self, n: u64) -> AsyncTiffResult<Vec<u8>> {
        let (buf, _endianness) = self.read(n).await?.into_inner();
        Ok(buf.into())
    }

    /// Read `n` i8s from the cursor, advancing the internal state by `n` bytes.
    pub(crate) async fn read_i8_n(&mut self, n: u64) -> AsyncTiffResult<Value> {
        let (buf, _endianness) = self.read(n).await?.into_inner();
        let values: &[i8] = bytemuck::try_cast_slice(&buf)?;
        Ok(Value::List(
            values.iter().copied().map(Value::SignedByte).collect(),
        ))
    }

    /// Read a u16 from the cursor, advancing the internal state by 2 bytes.
    pub(crate) async fn read_u16(&mut self) -> AsyncTiffResult<u16> {
        self.read(2).await?.read_u16()
    }

    /// Read `n` u16s from the cursor, advancing the internal state by `n * 2` bytes.
    pub(crate) async fn read_u16_n(&mut self, n: u64) -> AsyncTiffResult<Value> {
        let mut reader = self.read(n * 2).await?;

        // If the endianness matches the machine endianness, we can do a direct cast.
        if self.endianness == MACHINE_ENDIANNESS {
            let (buf, _endianness) = reader.into_inner();
            let values: &[u16] = bytemuck::try_cast_slice(&buf)?;
            Ok(Value::List(
                values.iter().copied().map(Value::Short).collect(),
            ))
        } else {
            let mut v = Vec::with_capacity(n as _);
            for _ in 0..n {
                v.push(Value::Short(reader.read_u16()?))
            }
            Ok(Value::List(v))
        }
    }

    /// Read `n` i16s from the cursor, advancing the internal state by `n * 2` bytes.
    pub(crate) async fn read_i16_n(&mut self, n: u64) -> AsyncTiffResult<Value> {
        let mut reader = self.read(n * 2).await?;

        // If the endianness matches the machine endianness, we can do a direct cast.
        if self.endianness == MACHINE_ENDIANNESS {
            let (buf, _endianness) = reader.into_inner();
            let values: &[i16] = bytemuck::try_cast_slice(&buf)?;
            Ok(Value::List(
                values.iter().copied().map(Value::SignedShort).collect(),
            ))
        } else {
            let mut v = Vec::with_capacity(n as _);
            for _ in 0..n {
                v.push(Value::SignedShort(reader.read_i16()?))
            }
            Ok(Value::List(v))
        }
    }

    /// Read a u32 from the cursor, advancing the internal state by 4 bytes.
    pub(crate) async fn read_u32(&mut self) -> AsyncTiffResult<u32> {
        self.read(4).await?.read_u32()
    }

    /// Read `n` u32s from the cursor, advancing the internal state by `n * 4` bytes.
    pub(crate) async fn read_u32_n(&mut self, n: u64) -> AsyncTiffResult<Value> {
        let mut reader = self.read(n * 4).await?;

        // If the endianness matches the machine endianness, we can do a direct cast.
        if self.endianness == MACHINE_ENDIANNESS {
            let (buf, _endianness) = reader.into_inner();
            let values: &[u32] = bytemuck::try_cast_slice(&buf)?;
            Ok(Value::List(
                values.iter().copied().map(Value::Unsigned).collect(),
            ))
        } else {
            let mut v = Vec::with_capacity(n as _);
            for _ in 0..n {
                v.push(Value::Unsigned(reader.read_u32()?))
            }
            Ok(Value::List(v))
        }
    }

    /// Read `n` Value::IFDs from the cursor, advancing the internal state by `n * 4` bytes.
    pub(crate) async fn read_ifd_n(&mut self, n: u64) -> AsyncTiffResult<Value> {
        let mut reader = self.read(n * 4).await?;

        // If the endianness matches the machine endianness, we can do a direct cast.
        if self.endianness == MACHINE_ENDIANNESS {
            let (buf, _endianness) = reader.into_inner();
            let values: &[u32] = bytemuck::try_cast_slice(&buf)?;
            Ok(Value::List(
                values.iter().copied().map(Value::Ifd).collect(),
            ))
        } else {
            let mut v = Vec::with_capacity(n as _);
            for _ in 0..n {
                v.push(Value::Ifd(reader.read_u32()?))
            }
            Ok(Value::List(v))
        }
    }

    /// Read a i32 from the cursor, advancing the internal state by 4 bytes.
    pub(crate) async fn read_i32(&mut self) -> AsyncTiffResult<i32> {
        self.read(4).await?.read_i32()
    }

    /// Read `n` i32s from the cursor, advancing the internal state by `n * 4` bytes.
    pub(crate) async fn read_i32_n(&mut self, n: u64) -> AsyncTiffResult<Value> {
        let mut reader = self.read(n * 4).await?;

        // If the endianness matches the machine endianness, we can do a direct cast.
        if self.endianness == MACHINE_ENDIANNESS {
            let (buf, _endianness) = reader.into_inner();
            let values: &[i32] = bytemuck::try_cast_slice(&buf)?;
            Ok(Value::List(
                values.iter().copied().map(Value::Signed).collect(),
            ))
        } else {
            let mut v = Vec::with_capacity(n as _);
            for _ in 0..n {
                v.push(Value::Signed(reader.read_i32()?))
            }
            Ok(Value::List(v))
        }
    }

    /// Read a u64 from the cursor, advancing the internal state by 8 bytes.
    pub(crate) async fn read_u64(&mut self) -> AsyncTiffResult<u64> {
        self.read(8).await?.read_u64()
    }

    /// Read `n` u64s from the cursor, advancing the internal state by `n * 8` bytes.
    pub(crate) async fn read_u64_n(&mut self, n: u64) -> AsyncTiffResult<Value> {
        let mut reader = self.read(n * 8).await?;

        // If the endianness matches the machine endianness, we can do a direct cast.
        if self.endianness == MACHINE_ENDIANNESS {
            let (buf, _endianness) = reader.into_inner();
            let values: &[u64] = bytemuck::try_cast_slice(&buf)?;
            Ok(Value::List(
                values.iter().copied().map(Value::UnsignedBig).collect(),
            ))
        } else {
            let mut v = Vec::with_capacity(n as _);
            for _ in 0..n {
                v.push(Value::UnsignedBig(reader.read_u64()?))
            }
            Ok(Value::List(v))
        }
    }

    /// Read `n` u64s from the cursor, advancing the internal state by `n * 8` bytes.
    pub(crate) async fn read_ifd8_n(&mut self, n: u64) -> AsyncTiffResult<Value> {
        let mut reader = self.read(n * 8).await?;

        // If the endianness matches the machine endianness, we can do a direct cast.
        if self.endianness == MACHINE_ENDIANNESS {
            let (buf, _endianness) = reader.into_inner();
            let values: &[u64] = bytemuck::try_cast_slice(&buf)?;
            Ok(Value::List(
                values.iter().copied().map(Value::IfdBig).collect(),
            ))
        } else {
            let mut v = Vec::with_capacity(n as _);
            for _ in 0..n {
                v.push(Value::IfdBig(reader.read_u64()?))
            }
            Ok(Value::List(v))
        }
    }

    /// Read a i64 from the cursor, advancing the internal state by 8 bytes.
    pub(crate) async fn read_i64(&mut self) -> AsyncTiffResult<i64> {
        self.read(8).await?.read_i64()
    }

    /// Read `n` i64s from the cursor, advancing the internal state by `n * 8` bytes.
    pub(crate) async fn read_i64_n(&mut self, n: u64) -> AsyncTiffResult<Value> {
        let mut reader = self.read(n * 8).await?;

        // If the endianness matches the machine endianness, we can do a direct cast.
        if self.endianness == MACHINE_ENDIANNESS {
            let (buf, _endianness) = reader.into_inner();
            let values: &[i64] = bytemuck::try_cast_slice(&buf)?;
            Ok(Value::List(
                values.iter().copied().map(Value::SignedBig).collect(),
            ))
        } else {
            let mut v = Vec::with_capacity(n as _);
            for _ in 0..n {
                v.push(Value::SignedBig(reader.read_i64()?))
            }
            Ok(Value::List(v))
        }
    }

    /// Read `n` f32s from the cursor, advancing the internal state by `n * 4` bytes.
    pub(crate) async fn read_f32_n(&mut self, n: u64) -> AsyncTiffResult<Value> {
        let mut reader = self.read(n * 4).await?;

        // If the endianness matches the machine endianness, we can do a direct cast.
        if self.endianness == MACHINE_ENDIANNESS {
            let (buf, _endianness) = reader.into_inner();
            let values: &[f32] = bytemuck::try_cast_slice(&buf)?;
            Ok(Value::List(
                values.iter().copied().map(Value::Float).collect(),
            ))
        } else {
            let mut v = Vec::with_capacity(n as _);
            for _ in 0..n {
                v.push(Value::Float(reader.read_f32()?))
            }
            Ok(Value::List(v))
        }
    }

    pub(crate) async fn read_f64(&mut self) -> AsyncTiffResult<f64> {
        self.read(8).await?.read_f64()
    }

    /// Read `n` f64s from the cursor, advancing the internal state by `n * 8` bytes.
    pub(crate) async fn read_f64_n(&mut self, n: u64) -> AsyncTiffResult<Value> {
        let mut reader = self.read(n * 8).await?;

        // If the endianness matches the machine endianness, we can do a direct cast.
        if self.endianness == MACHINE_ENDIANNESS {
            let (buf, _endianness) = reader.into_inner();
            let values: &[f64] = bytemuck::try_cast_slice(&buf)?;
            Ok(Value::List(
                values.iter().copied().map(Value::Double).collect(),
            ))
        } else {
            let mut v = Vec::with_capacity(n as _);
            for _ in 0..n {
                v.push(Value::Double(reader.read_f64()?))
            }
            Ok(Value::List(v))
        }
    }
}
