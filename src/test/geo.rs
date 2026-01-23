use std::path::PathBuf;
use std::sync::Arc;

use object_store::local::LocalFileSystem;

use crate::metadata::cache::ReadaheadMetadataCache;
use crate::metadata::TiffMetadataReader;
use crate::reader::{AsyncFileReader, ObjectReader};

#[tokio::test]
async fn test_parse_file_with_unknown_geokey() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path =
        object_store::path::Path::parse("fixtures/other/geogtowgs_subset_USGS_13_s14w171.tif")
            .unwrap();
    let store = Arc::new(LocalFileSystem::new_with_prefix(&manifest_dir).unwrap());
    let reader = Arc::new(ObjectReader::new(store, path)) as Arc<dyn AsyncFileReader>;
    let prefetch_reader = ReadaheadMetadataCache::new(reader.clone());
    let mut metadata_reader = TiffMetadataReader::try_open(&prefetch_reader)
        .await
        .unwrap();
    let _ = metadata_reader
        .read_all_ifds(&prefetch_reader)
        .await
        .unwrap();
}
