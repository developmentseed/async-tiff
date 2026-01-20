use crate::ifd::ImageFileDirectory;
use crate::reader::Endianness;

/// A TIFF file.
#[derive(Debug, Clone)]
pub struct TIFF {
    endianness: Endianness,
    ifds: Vec<ImageFileDirectory>,
}

impl TIFF {
    /// Create a new TIFF from existing IFDs.
    pub fn new(ifds: Vec<ImageFileDirectory>, endianness: Endianness) -> Self {
        Self { ifds, endianness }
    }

    /// Access the underlying Image File Directories.
    pub fn ifds(&self) -> &[ImageFileDirectory] {
        &self.ifds
    }

    /// Get the endianness of the TIFF file.
    pub fn endianness(&self) -> Endianness {
        self.endianness
    }
}

#[cfg(test)]
mod test {
    use std::io::BufReader;
    use std::sync::Arc;

    use object_store::local::LocalFileSystem;
    use tiff::decoder::{DecodingResult, Limits};

    use super::*;
    use crate::metadata::cache::ReadaheadMetadataCache;
    use crate::metadata::TiffMetadataReader;
    use crate::reader::{AsyncFileReader, ObjectReader};

    #[ignore = "local file"]
    #[tokio::test]
    async fn tmp() {
        let folder = "/Users/kyle/github/developmentseed/async-tiff/";
        let path = object_store::path::Path::parse("m_4007307_sw_18_060_20220803.tif").unwrap();
        let store = Arc::new(LocalFileSystem::new_with_prefix(folder).unwrap());
        let reader = Arc::new(ObjectReader::new(store, path)) as Arc<dyn AsyncFileReader>;
        let cached_reader = ReadaheadMetadataCache::new(reader.clone());
        let mut metadata_reader = TiffMetadataReader::try_open(&cached_reader).await.unwrap();
        let ifds = metadata_reader.read_all_ifds(&cached_reader).await.unwrap();
        let tiff = TIFF::new(ifds, metadata_reader.endianness());

        let ifd = &tiff.ifds[1];
        let tile = ifd.fetch_tile(0, 0, reader.as_ref()).await.unwrap();
        let array = tile.decode(&Default::default()).unwrap();
        std::fs::write("img.buf", array.raw_data()).unwrap();
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
