use crate::ifd;
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
    let tile = ifd.fetch_tile(0, 0, None, &reader).await.unwrap();

    let array = tile.decode(&Default::default()).unwrap();

    // For planar configuration, shape is [bands, height, width]
    assert_eq!(array.shape(), [3, 256, 256]);

    // Verify we have the correct amount of data
    assert_eq!(array.data().len(), 3 * 256 * 256);
}

#[tokio::test]
async fn test_band_interleaved_single_tile_with_specific_bands() {
    let filename = "geotiff-test-data/real_data/eox/eox_cloudless.tif";

    let (reader, tiff) = open_tiff(filename).await;
    let ifd = &tiff.ifds()[0];

    // Fetch tile at position (0, 0) - only first two bands
    let tile = ifd
        .fetch_tile(0, 0, Some(ifd::FetchOptions::new(vec![0, 1])), &reader)
        .await
        .unwrap();
    let array = tile.decode(&Default::default()).unwrap();

    // For planar configuration, shape is [bands, height, width]
    assert_eq!(array.shape(), [2, 256, 256]);

    // Verify we have the correct amount of data
    assert_eq!(array.data().len(), 2 * 256 * 256);
}

#[tokio::test]
async fn test_band_interleaved_multi_tiles_with_specific_bands() {
    let filename = "geotiff-test-data/real_data/eox/eox_cloudless.tif";

    let (reader, tiff) = open_tiff(filename).await;
    let ifd = &tiff.ifds()[0];

    // Fetch tiles at position (0, 0) and (1, 0) - only last two bands
    let tiles = ifd
        .fetch_tiles(
            &[(0, 0), (1, 0)],
            Some(ifd::FetchOptions::new(vec![1, 2])),
            &reader,
        )
        .await
        .unwrap();

    for tile in tiles {
        let array = tile.decode(&Default::default()).unwrap();

        // For planar configuration, shape is [bands, height, width]
        assert_eq!(array.shape(), [2, 256, 256]);

        // Verify we have the correct amount of data
        assert_eq!(array.data().len(), 2 * 256 * 256);
    }
}
