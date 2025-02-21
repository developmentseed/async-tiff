use bytes::Bytes;

use crate::async_reader::{AsyncCursor, Endianness};
use crate::error::Result;
use crate::ifd::ImageFileDirectories;
use crate::AsyncFileReader;

pub struct COGReader {
    #[allow(dead_code)]
    reader: Box<dyn AsyncFileReader>,
    ifds: ImageFileDirectories,
}

impl COGReader {
    pub async fn try_open(reader: Box<dyn AsyncFileReader>) -> Result<Self> {
        let mut cursor = AsyncCursor::new(reader);
        let magic_bytes = cursor.read(2).await;
        // Should be b"II" for little endian or b"MM" for big endian
        if magic_bytes == Bytes::from_static(b"II") {
            cursor.set_endianness(Endianness::LittleEndian);
        } else if magic_bytes == Bytes::from_static(b"MM") {
            cursor.set_endianness(Endianness::BigEndian);
        } else {
            panic!("unexpected magic bytes {magic_bytes:?}");
        }

        let version = cursor.read_u16().await;

        // Assert it's a standard non-big tiff
        assert_eq!(version, 42);

        let first_ifd_location = cursor.read_u32().await;

        let ifds = ImageFileDirectories::open(&mut cursor, first_ifd_location as usize)
            .await
            .unwrap();

        let reader = cursor.into_inner();
        Ok(Self { reader, ifds })
    }

    /// Return the EPSG code representing the crs of the image
    pub fn epsg(&self) -> Option<u16> {
        let ifd = &self.ifds.as_ref()[0];
        ifd.geo_key_directory
            .as_ref()
            .and_then(|gkd| gkd.epsg_code())
    }

    /// Return the bounds of the image in native crs
    pub fn native_bounds(&self) -> Option<(f64, f64, f64, f64)> {
        let ifd = &self.ifds.as_ref()[0];
        ifd.native_bounds()
    }
}

#[cfg(test)]
mod test {
    use std::io::BufReader;
    use std::sync::Arc;

    use crate::ObjectReader;

    use super::*;
    use object_store::local::LocalFileSystem;
    use tiff::decoder::{DecodingResult, Limits};

    #[ignore = "local file"]
    #[tokio::test]
    async fn tmp() {
        let folder = "/Users/kyle/github/developmentseed/async-tiff/";
        let path = object_store::path::Path::parse("m_4007307_sw_18_060_20220803.tif").unwrap();
        let store = Arc::new(LocalFileSystem::new_with_prefix(folder).unwrap());
        let reader = ObjectReader::new(store, path);
        let cog_reader = COGReader::try_open(Box::new(reader.clone())).await.unwrap();
        let ifd = &cog_reader.ifds.as_ref()[4];
        dbg!(ifd.compression);
        let tile = ifd.get_tile(0, 0, Box::new(reader)).await.unwrap();
        std::fs::write("img.buf", tile).unwrap();
        // dbg!(tile.len());
    }

    #[ignore = "local file"]
    #[test]
    fn tmp_tiff_example() {
        let path = "/Users/kyle/github/developmentseed/async-tiff/m_4007307_sw_18_060_20220803.tif";
        let reader = std::fs::File::open(path).unwrap();
        let mut decoder = tiff::decoder::Decoder::new(BufReader::new(reader))
            .unwrap()
            .with_limits(Limits::unlimited());
        let result = decoder.read_image().unwrap();
        match result {
            DecodingResult::U8(content) => std::fs::write("img_from_tiff.buf", content).unwrap(),
            _ => todo!(),
        }
    }
}
