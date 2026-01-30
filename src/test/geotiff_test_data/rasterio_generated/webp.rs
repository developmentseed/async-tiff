use crate::tags::PhotometricInterpretation;
use crate::test::util::open_tiff;
use crate::{DataType, TypedArray};

#[tokio::test]
async fn test_uint8_rgba_webp() {
    let filename = "geotiff-test-data/rasterio_generated/fixtures/uint8_rgba_webp_block64_cog.tif";
    let (reader, tiff) = open_tiff(filename).await;
    let ifd = &tiff.ifds()[0];
    assert_eq!(ifd.image_height(), 128);
    assert_eq!(ifd.image_width(), 128);
    assert_eq!(
        ifd.photometric_interpretation(),
        PhotometricInterpretation::RGB
    );
    assert_eq!(ifd.tile_width(), Some(64));
    assert_eq!(ifd.tile_height(), Some(64));

    let tile: crate::Tile = ifd.fetch_tile(0, 0, &reader).await.unwrap();

    let array = tile.decode(&Default::default()).unwrap();

    assert_eq!(array.shape, [64, 64, 4]);
    assert_eq!(array.data_type, Some(DataType::UInt8));
    assert!(matches!(array.data, TypedArray::UInt8(_)));
}

#[tokio::test]
async fn test_uint8_rgb_webp() {
    let filename = "geotiff-test-data/rasterio_generated/fixtures/uint8_rgba_webp_block64_cog.tif";
    let (reader, tiff) = open_tiff(filename).await;
    let ifd = &tiff.ifds()[0];
    assert_eq!(ifd.image_height(), 128);
    assert_eq!(ifd.image_width(), 128);
    assert_eq!(
        ifd.photometric_interpretation(),
        PhotometricInterpretation::RGB
    );
    assert_eq!(ifd.tile_width(), Some(64));
    assert_eq!(ifd.tile_height(), Some(64));

    let tile: crate::Tile = ifd.fetch_tile(0, 0, &reader).await.unwrap();

    let array = tile.decode(&Default::default()).unwrap();

    assert_eq!(array.shape, [64, 64, 4]);
    assert_eq!(array.data_type, Some(DataType::UInt8));
    assert!(matches!(array.data, TypedArray::UInt8(_)));
}
