use std::env::current_dir;
use std::sync::Arc;

use async_tiff::{COGReader, ObjectReader};
use object_store::local::LocalFileSystem;

const TEST_IMAGE_DIR: &str = "tests/image_tiff/images/";

pub(crate) async fn open_tiff(filename: &str) -> COGReader {
    let store = Arc::new(LocalFileSystem::new_with_prefix(current_dir().unwrap()).unwrap());
    let path = format!("{TEST_IMAGE_DIR}/{filename}");
    let reader = Box::new(ObjectReader::new(store.clone(), path.as_str().into()));
    COGReader::try_open(reader).await.unwrap()
}
