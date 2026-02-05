use crate::tags::PhotometricInterpretation;
use crate::test::util::open_tiff;
use crate::{DataType, TypedArray};

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
    assert_eq!(ifd.tile_width(), Some(256));
    assert_eq!(ifd.tile_height(), Some(256));

    let tile = ifd.fetch_tile(0, 0, &reader).await.unwrap();

    let array = tile.decode(&Default::default()).unwrap();

    assert_eq!(array.shape, [64, 64, 3])
}
