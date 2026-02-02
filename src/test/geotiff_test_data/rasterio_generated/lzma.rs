use crate::tags::PhotometricInterpretation;
use crate::test::util::open_tiff;
use crate::{DataType, TypedArray};

#[tokio::test]
async fn test_uint8() {
    let filename = "geotiff-test-data/rasterio_generated/fixtures/uint8_1band_lzma_block64.tif";
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

    let tile: crate::Tile = ifd.fetch_tile(0, 0, &reader).await.unwrap();

    let array = tile.decode(&Default::default()).unwrap();

    assert_eq!(array.shape, [64, 64, 1]);
    assert_eq!(array.data_type, Some(DataType::UInt8));
    assert!(matches!(array.data, TypedArray::UInt8(_)));
}
