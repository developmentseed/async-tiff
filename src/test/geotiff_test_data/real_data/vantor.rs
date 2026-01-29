use crate::tags::PhotometricInterpretation;
use crate::test::util::open_tiff;
use crate::{DataType, TypedArray};

#[tokio::test]
async fn test_vantor_opendata_yellowstone() {
    let filename = "geotiff-test-data/real_data/vantor/maxar_opendata_yellowstone_visual.tif";

    let (reader, tiff) = open_tiff(filename).await;
    let ifd = &tiff.ifds()[0];
    assert_eq!(ifd.image_height(), 128);
    assert_eq!(ifd.image_width(), 128);
    assert_eq!(
        ifd.photometric_interpretation(),
        PhotometricInterpretation::YCbCr
    );
    assert_eq!(ifd.tile_width(), Some(64));
    assert_eq!(ifd.tile_height(), Some(64));

    let tile: crate::Tile = ifd.fetch_tile(0, 0, &reader).await.unwrap();

    let array = tile.decode(&Default::default()).unwrap();

    assert_eq!(array.shape, [64, 64, 3])
}

#[tokio::test]
async fn test_load_single_bit_mask() {
    let filename = "geotiff-test-data/real_data/vantor/maxar_opendata_yellowstone_visual.tif";

    let (reader, tiff) = open_tiff(filename).await;
    let ifd = &tiff.ifds()[1]; // full-resolution mask
    assert_eq!(ifd.image_height(), 128);
    assert_eq!(ifd.image_width(), 128);
    assert_eq!(
        ifd.photometric_interpretation(),
        PhotometricInterpretation::TransparencyMask
    );
    assert_eq!(ifd.samples_per_pixel(), 1);
    assert_eq!(ifd.bits_per_sample(), &[1]);
    assert_eq!(ifd.tile_width(), Some(64));
    assert_eq!(ifd.tile_height(), Some(64));

    let tile: crate::Tile = ifd.fetch_tile(0, 0, &reader).await.unwrap();

    let array = tile.decode(&Default::default()).unwrap();

    assert_eq!(array.shape, [64, 64, 1]);
    assert_eq!(array.data_type, Some(DataType::Bool));

    match array.data {
        TypedArray::Bool(data) => {
            assert_eq!(data.len(), 64 * 64);
        }
        _ => panic!("Expected Bool typed array"),
    };
}

#[tokio::test]
async fn test_vantor_opendata_yellowstone_overview() {
    let filename = "geotiff-test-data/real_data/vantor/maxar_opendata_yellowstone_visual.tif";

    let (reader, tiff) = open_tiff(filename).await;
    let ifd = &tiff.ifds()[2]; // overview
    assert_eq!(ifd.image_height(), 64);
    assert_eq!(ifd.image_width(), 64);
    assert_eq!(
        ifd.photometric_interpretation(),
        PhotometricInterpretation::YCbCr
    );
    assert_eq!(ifd.tile_width(), Some(64));
    assert_eq!(ifd.tile_height(), Some(64));

    let tile: crate::Tile = ifd.fetch_tile(0, 0, &reader).await.unwrap();

    let array = tile.decode(&Default::default()).unwrap();

    assert_eq!(array.shape, [64, 64, 3])
}
