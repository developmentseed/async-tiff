use std::env;
use std::sync::Arc;

use async_tiff::metadata::{PrefetchBuffer, TiffMetadataReader};
use async_tiff::reader::{AsyncFileReader, ObjectReader};
use object_store::local::LocalFileSystem;

#[tokio::test]
async fn test_parse_file_with_unknown_geokey() {
    let folder = env::current_dir().unwrap();
    let path = object_store::path::Path::parse("tests/images/geogtowgs_subset_USGS_13_s14w171.tif")
        .unwrap();
    let store = Arc::new(LocalFileSystem::new_with_prefix(folder).unwrap());
    let reader = Arc::new(ObjectReader::new(store, path)) as Arc<dyn AsyncFileReader>;
    let prefetch_reader = PrefetchBuffer::new(reader.clone(), 32 * 1024)
        .await
        .unwrap();
    let mut metadata_reader = TiffMetadataReader::try_open(&prefetch_reader)
        .await
        .unwrap();
    let _ = metadata_reader
        .read_all_ifds(&prefetch_reader)
        .await
        .unwrap();
}
