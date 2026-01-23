use std::env::current_dir;
use std::sync::Arc;

use object_store::local::LocalFileSystem;

use crate::metadata::TiffMetadataReader;
use crate::reader::{AsyncFileReader, ObjectReader};
use crate::TIFF;

const TEST_IMAGE_DIR: &str = "fixtures/image-tiff/";

pub(crate) async fn open_tiff(filename: &str) -> TIFF {
    let store = Arc::new(LocalFileSystem::new_with_prefix(current_dir().unwrap()).unwrap());
    let path = format!("{TEST_IMAGE_DIR}/{filename}");
    let reader = Arc::new(ObjectReader::new(store.clone(), path.as_str().into()))
        as Arc<dyn AsyncFileReader>;
    let mut metadata_reader = TiffMetadataReader::try_open(&reader).await.unwrap();
    metadata_reader.read(&reader).await.unwrap()
}
