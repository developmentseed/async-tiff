use std::sync::Arc;

use async_tiff::metadata::{PrefetchBuffer, TiffMetadataReader};
use async_tiff::reader::{AsyncFileReader, ObjectReader};
use async_tiff::TIFF;
use reqwest::Url;

pub(crate) async fn open_remote_tiff(url: &str) -> TIFF {
    let parsed_url = Url::parse(url).expect("failed parsing url");
    let (store, path) = object_store::parse_url(&parsed_url).unwrap();

    let reader = Arc::new(ObjectReader::new(Arc::new(store), path)) as Arc<dyn AsyncFileReader>;
    let prefetch_reader = PrefetchBuffer::new(reader.clone(), 32 * 1024)
        .await
        .unwrap();
    let mut metadata_reader = TiffMetadataReader::try_open(&prefetch_reader)
        .await
        .unwrap();
    let ifds = metadata_reader
        .read_all_ifds(&prefetch_reader)
        .await
        .unwrap();
    TIFF::new(ifds)
}
