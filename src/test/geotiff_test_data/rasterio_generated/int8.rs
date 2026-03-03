use crate::tags::PhotometricInterpretation;
use crate::test::util::open_tiff;
use crate::{ifd, DataType, TypedArray};

#[tokio::test]
async fn test_fetch_some_bands() {
    let filename = "geotiff-test-data/rasterio_generated/fixtures/int8_3band_zstd_block64.tif";
    let (reader, tiff) = open_tiff(filename).await;
    let ifd = &tiff.ifds()[0];

    assert_eq!(ifd.image_height(), 128);
    assert_eq!(ifd.image_width(), 128);
    assert_eq!(
        ifd.photometric_interpretation(),
        PhotometricInterpretation::BlackIsZero
    );
    assert_eq!(ifd.tile_width(), Some(64));
    assert_eq!(ifd.tile_height(), Some(64));

    let tile = ifd
        .fetch_tile(0, 0, Some(ifd::ReadOptions::new(vec![0, 2])), &reader)
        .await
        .unwrap();
    let array = tile.decode(&Default::default()).unwrap();

    assert_eq!(array.shape, [2, 64, 64]); // channel, height, width
    assert_eq!(array.data_type, Some(DataType::Int8));
    assert!(matches!(array.data, TypedArray::Int8(_)));
}

#[tokio::test]
async fn test_fetch_invalid_band() {
    let filename = "geotiff-test-data/rasterio_generated/fixtures/int8_3band_zstd_block64.tif";
    let (reader, tiff) = open_tiff(filename).await;
    let ifd = &tiff.ifds()[0];

    let result = ifd
        .fetch_tile(0, 0, Some(ifd::ReadOptions::new(vec![3])), &reader) // max is 3 bands, start indexing from 0
        .await;
    assert_eq!(
        result.err().unwrap().to_string(),
        "General error: band 3 is greater than 2 (0-based indexing)"
    );
}
