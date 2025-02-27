use crate::async_reader::AsyncCursor;
use crate::error::Result;
use crate::ifd::ImageFileDirectories;
use crate::tiff::{TiffError, TiffFormatError};
use crate::AsyncFileReader;

#[derive(Debug)]
pub struct COGReader {
    #[allow(dead_code)]
    cursor: AsyncCursor,
    ifds: ImageFileDirectories,
    #[allow(dead_code)]
    bigtiff: bool,
}

impl COGReader {
    pub async fn try_open(reader: Box<dyn AsyncFileReader>) -> Result<Self> {
        let mut cursor = AsyncCursor::try_open_tiff(reader).await?;
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

        let ifds = ImageFileDirectories::open(&mut cursor, first_ifd_location, bigtiff).await?;

        Ok(Self {
            cursor,
            ifds,
            bigtiff,
        })
    }

    pub fn ifds(&self) -> &ImageFileDirectories {
        &self.ifds
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

    use crate::decoder::DecoderRegistry;
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

        let ifd = &cog_reader.ifds.as_ref()[1];
        let decoder_registry = DecoderRegistry::default();
        let tile = ifd
            .get_tile(0, 0, Box::new(reader), &decoder_registry)
            .await
            .unwrap();
        std::fs::write("img.buf", tile).unwrap();
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
