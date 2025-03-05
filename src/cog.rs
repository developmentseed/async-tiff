use crate::error::AsyncTiffResult;
use crate::ifd::ImageFileDirectories;
use crate::reader::{AsyncCursor, AsyncFileReader};
use crate::tiff::{TiffError, TiffFormatError};

/// A TIFF file.
#[derive(Debug, Clone)]
pub struct TIFF {
    ifds: ImageFileDirectories,
}

impl TIFF {
    /// Open a new TIFF file.
    ///
    /// This will read all the Image File Directories (IFDs) in the file.
    pub async fn try_open(reader: Box<dyn AsyncFileReader>) -> AsyncTiffResult<Self> {
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

        Ok(Self { ifds })
    }

    /// Access the underlying Image File Directories.
    pub fn ifds(&self) -> &ImageFileDirectories {
        &self.ifds
    }
}

#[cfg(test)]
mod test {
    use std::io::BufReader;
    use std::sync::Arc;

    use crate::decoder::DecoderRegistry;
    use crate::reader::ObjectReader;

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

        let cog_reader = TIFF::try_open(Box::new(reader.clone())).await.unwrap();

        let ifd = &cog_reader.ifds.as_ref()[1];
        let decoder_registry = DecoderRegistry::default();
        let tile = ifd.fetch_tile(0, 0, &reader).await.unwrap();
        let tile = tile.decode(&decoder_registry).unwrap();
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
