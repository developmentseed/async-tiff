use crate::tags::{PhotometricInterpretation, PlanarConfiguration};
use crate::test::util::open_tiff;

#[tokio::test]
async fn test_band_interleaved() {
    let filename = "geotiff-test-data/real_data/eox/eox_cloudless.tif";

    let (reader, tiff) = open_tiff(filename).await;
    let ifd = &tiff.ifds()[0];
    assert_eq!(ifd.image_height(), 256);
    assert_eq!(ifd.image_width(), 512);
    assert_eq!(
        ifd.photometric_interpretation(),
        PhotometricInterpretation::RGB
    );
    assert_eq!(ifd.planar_configuration(), PlanarConfiguration::Planar);
    assert_eq!(ifd.tile_width(), Some(256));
    assert_eq!(ifd.tile_height(), Some(256));

    // Fetch tile at position (0, 0) - this fetches all 3 bands automatically
    let tile = ifd.fetch_tile(0, 0, &reader).await.unwrap();

    let array = tile.decode(&Default::default()).unwrap();

    // For planar configuration, shape is [bands, height, width]
    assert_eq!(array.shape(), [3, 256, 256]);

    // Verify we have the correct amount of data
    assert_eq!(array.data().len(), 3 * 256 * 256);
}
