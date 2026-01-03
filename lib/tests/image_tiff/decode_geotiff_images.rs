extern crate tiff;

use crate::image_tiff::util::open_tiff;

#[tokio::test]
async fn test_geo_tiff() {
    let filenames = ["geo-5b.tif"];
    for filename in filenames.iter() {
        let tiff = open_tiff(filename).await;
        let ifd = &tiff.ifds()[0];
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
