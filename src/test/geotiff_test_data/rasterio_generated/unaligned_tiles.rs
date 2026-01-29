use crate::tags::PhotometricInterpretation;
use crate::test::util::open_tiff;

#[tokio::test]
async fn test_unaligned() {
    let filename =
        "geotiff-test-data/rasterio_generated/fixtures/uint8_1band_deflate_block128_unaligned.tif";
    let (reader, tiff) = open_tiff(filename).await;
    let ifd = &tiff.ifds()[0];
    assert_eq!(ifd.image_height(), 266);
    assert_eq!(ifd.image_width(), 265);
    assert_eq!(
        ifd.photometric_interpretation(),
        PhotometricInterpretation::BlackIsZero
    );
    assert_eq!(ifd.tile_width(), Some(128));
    assert_eq!(ifd.tile_height(), Some(128));

    let tile: crate::Tile = ifd.fetch_tile(2, 0, &reader).await.unwrap();

    let array = tile.decode(&Default::default()).unwrap();

    assert_eq!(array.shape, [128, 128, 1])
}
