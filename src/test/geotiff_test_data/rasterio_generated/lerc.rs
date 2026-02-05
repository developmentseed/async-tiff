use crate::tags::Tag;
use crate::test::util::open_tiff;
use crate::{DataType, TypedArray};

#[tokio::test]
async fn test_lerc() {
    let filename = "geotiff-test-data/rasterio_generated/fixtures/float32_1band_lerc_block32.tif";
    let (reader, tiff) = open_tiff(filename).await;
    let ifd = &tiff.ifds()[0];

    assert_eq!(ifd.image_height(), 128);
    assert_eq!(ifd.image_width(), 128);
    assert_eq!(ifd.tile_width(), Some(64));
    assert_eq!(ifd.tile_height(), Some(64));

    let tile = ifd.fetch_tile(0, 0, &reader).await.unwrap();

    let array = tile.decode(&Default::default()).unwrap();

    assert_eq!(array.shape, [64, 64, 1]);
    assert_eq!(array.data_type, Some(DataType::Float32));
    assert!(matches!(array.data, TypedArray::Float32(_)));
}

#[tokio::test]
async fn test_lerc_deflate() {
    let filename =
        "geotiff-test-data/rasterio_generated/fixtures/float32_1band_lerc_deflate_block32.tif";
    let (reader, tiff) = open_tiff(filename).await;
    let ifd = &tiff.ifds()[0];

    let lerc_params = ifd
        .other_tags()
        .get(&Tag::LercParameters)
        .unwrap()
        .clone()
        .into_u32_vec()
        .unwrap();

    let lerc_version = lerc_params[0];
    assert_eq!(lerc_version, 4);

    let lerc_compression = lerc_params[1];
    assert_eq!(lerc_compression, 1); // 1 = deflate

    assert_eq!(ifd.image_height(), 128);
    assert_eq!(ifd.image_width(), 128);
    assert_eq!(ifd.tile_width(), Some(64));
    assert_eq!(ifd.tile_height(), Some(64));

    let tile = ifd.fetch_tile(0, 0, &reader).await.unwrap();

    // TODO: implement decoding
    // let array = tile.decode(&Default::default()).unwrap();

    // assert_eq!(array.shape, [64, 64, 1]);
    // assert_eq!(array.data_type, Some(DataType::Float32));
    // assert!(matches!(array.data, TypedArray::Float32(_)));
}

#[tokio::test]
async fn test_lerc_zstd() {
    let filename =
        "geotiff-test-data/rasterio_generated/fixtures/float32_1band_lerc_zstd_block32.tif";
    let (reader, tiff) = open_tiff(filename).await;
    let ifd = &tiff.ifds()[0];

    let lerc_params = ifd
        .other_tags()
        .get(&Tag::LercParameters)
        .unwrap()
        .clone()
        .into_u32_vec()
        .unwrap();

    let lerc_version = lerc_params[0];
    assert_eq!(lerc_version, 4);

    let lerc_compression = lerc_params[1];
    assert_eq!(lerc_compression, 2); // 2 = zstd

    assert_eq!(ifd.image_height(), 128);
    assert_eq!(ifd.image_width(), 128);
    assert_eq!(ifd.tile_width(), Some(64));
    assert_eq!(ifd.tile_height(), Some(64));

    let tile = ifd.fetch_tile(0, 0, &reader).await.unwrap();

    // TODO: implement decoding
    // let array = tile.decode(&Default::default()).unwrap();

    // assert_eq!(array.shape, [64, 64, 1]);
    // assert_eq!(array.data_type, Some(DataType::Float32));
    // assert!(matches!(array.data, TypedArray::Float32(_)));
}
