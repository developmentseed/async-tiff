use std::env::current_dir;
use std::sync::Arc;

use async_tiff::reader::ObjectReader;
use async_tiff::TIFF;
use object_store::local::LocalFileSystem;

const TEST_IMAGE_DIR: &str = "tests/image_tiff/images/";

pub(crate) async fn open_tiff(filename: &str) -> TIFF {
    let store = Arc::new(LocalFileSystem::new_with_prefix(current_dir().unwrap()).unwrap());
    let path = format!("{TEST_IMAGE_DIR}/{filename}");
    let reader = Arc::new(ObjectReader::new(store.clone(), path.as_str().into()));
    TIFF::try_open(reader).await.unwrap()
}
