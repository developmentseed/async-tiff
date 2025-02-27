extern crate tiff;

use async_tiff::{COGReader, ObjectReader};
use object_store::local::LocalFileSystem;

use std::env::current_dir;
use std::sync::Arc;

const TEST_IMAGE_DIR: &str = "tests/image_tiff/images";

// let folder = "/Users/kyle/github/developmentseed/async-tiff/";
// let path = object_store::path::Path::parse("m_4007307_sw_18_060_20220803.tif").unwrap();
// let store = Arc::new(LocalFileSystem::new_with_prefix(folder).unwrap());
// let reader = ObjectReader::new(store, path);

// let cog_reader = COGReader::try_open(Box::new(reader.clone())).await.unwrap();

// let ifd = &cog_reader.ifds.as_ref()[1];
// let decoder_registry = DecoderRegistry::default();
// let tile = ifd
//     .get_tile(0, 0, Box::new(reader), &decoder_registry)
//     .await
//     .unwrap();
// std::fs::write("img.buf", tile).unwrap();

#[tokio::test]
async fn test_geo_tiff() {
    let filenames = ["geo-5b.tif"];
    let store = Arc::new(LocalFileSystem::new_with_prefix(current_dir().unwrap()).unwrap());

    for filename in filenames.iter() {
        let path = format!("{TEST_IMAGE_DIR}/{filename}");
        let reader = ObjectReader::new(store.clone(), path.as_str().into());
        let image_reader = COGReader::try_open(Box::new(reader)).await.unwrap();
        let ifd = &image_reader.ifds().as_ref()[0];
        dbg!(&ifd);
        assert_eq!(ifd.image_height(), 10);
        assert_eq!(ifd.image_width(), 10);
        assert_eq!(ifd.bits_per_sample(), vec![16; 5]);
        assert_eq!(
            ifd.strip_offsets().expect("Cannot get StripOffsets"),
            vec![418]
        );
        assert_eq!(ifd.rows_per_strip().expect("Cannot get RowsPerStrip"), 10);
        assert_eq!(
            ifd.strip_byte_counts().expect("Cannot get StripByteCounts"),
            vec![1000]
        );
        assert_eq!(
            ifd.model_pixel_scale().expect("Cannot get pixel scale"),
            vec![60.0, 60.0, 0.0]
        );

        // We don't currently support reading strip images
        // let DecodingResult::I16(data) = decoder.read_image().unwrap() else {
        //     panic!("Cannot read band data")
        // };
        // assert_eq!(data.len(), 500);
    }
}
