use std::sync::Arc;

use crate::error::AsyncTiffResult;
use crate::ifd::ImageFileDirectory;
use crate::reader::AsyncFileReader;

/// A TIFF file.
#[derive(Debug, Clone)]
pub struct TIFF {
    ifds: Vec<ImageFileDirectory>,
}

impl TIFF {
    /// Create a new TIFF from existing IFDs.
    pub fn new(ifds: Vec<ImageFileDirectory>) -> Self {
        Self { ifds }
    }

    /// Open a new TIFF file.
    ///
    /// This will read all the Image File Directories (IFDs) in the file.
    pub async fn try_open(reader: Arc<dyn AsyncFileReader>) -> AsyncTiffResult<Self> {
        let ifds = reader.get_metadata().await?;
        Ok(Self { ifds })
    }

    /// Access the underlying Image File Directories.
    pub fn ifds(&self) -> &[ImageFileDirectory] {
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
        let reader = Arc::new(ObjectReader::new(store, path));

        let cog_reader = TIFF::try_open(reader.clone()).await.unwrap();

        let ifd = &cog_reader.ifds[1];
        let decoder_registry = DecoderRegistry::default();
        let tile = ifd.fetch_tile(0, 0, reader.as_ref()).await.unwrap();
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
