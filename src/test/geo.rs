use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use object_store::local::LocalFileSystem;

use crate::geo::{GeoKeyDirectory, GeoKeyTag};
use crate::metadata::cache::ReadaheadMetadataCache;
use crate::metadata::TiffMetadataReader;
use crate::reader::{AsyncFileReader, ObjectReader};
use crate::tag_value::TagValue;

#[test]
fn test_towgs84_and_rectified_grid_angle_keys() {
    let towgs84 = vec![598.1, 73.7, 418.2, 0.202, 0.045, -2.455, 6.7];
    let mut tags = HashMap::new();
    tags.insert(GeoKeyTag::ModelType, TagValue::Short(1));
    tags.insert(
        GeoKeyTag::GeogTOWGS84,
        TagValue::List(towgs84.iter().copied().map(TagValue::Double).collect()),
    );
    tags.insert(
        GeoKeyTag::ProjRectifiedGridAngle,
        TagValue::Double(323.130102361111),
    );

    let gkd = GeoKeyDirectory::from_tags(tags).unwrap();
    assert_eq!(gkd.geog_towgs84, Some(towgs84));
    assert_eq!(gkd.proj_rectified_grid_angle, Some(323.130102361111));
}

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
    let ifds = metadata_reader
        .read_all_ifds(&prefetch_reader)
        .await
        .unwrap();

    // This file carries a three-element GeogTOWGS84GeoKey (2062).
    let gkd = ifds[0].geo_key_directory().unwrap();
    assert_eq!(gkd.geog_towgs84, Some(vec![0.0, 0.0, 0.0]));
}
