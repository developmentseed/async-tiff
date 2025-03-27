use std::env::current_dir;
use std::sync::Arc;

use async_tiff::metadata::TiffMetadataReader;
use async_tiff::reader::{AsyncFileReader, ObjectReader};
use async_tiff::TIFF;
use object_store::local::LocalFileSystem;

const TEST_IMAGE_DIR: &str = "tests/image_tiff/images/";

pub(crate) async fn open_tiff(filename: &str) -> TIFF {
    let store = Arc::new(LocalFileSystem::new_with_prefix(current_dir().unwrap()).unwrap());
    let path = format!("{TEST_IMAGE_DIR}/{filename}");
    let reader = Arc::new(ObjectReader::new(store.clone(), path.as_str().into()))
        as Arc<dyn AsyncFileReader>;
    let mut metadata_reader = TiffMetadataReader::try_open(&reader).await.unwrap();
    let ifds = metadata_reader.read_all_ifds(&reader).await.unwrap();
    TIFF::new(ifds)
}
