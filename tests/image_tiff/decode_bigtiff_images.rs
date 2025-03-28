extern crate tiff;

use async_tiff::tiff::tags::PhotometricInterpretation;

use crate::image_tiff::util::open_tiff;

#[tokio::test]
async fn test_big_tiff() {
    let filenames = [
        "bigtiff/BigTIFF.tif",
        "bigtiff/BigTIFFMotorola.tif",
        "bigtiff/BigTIFFLong.tif",
    ];
    for filename in filenames.iter() {
        let tiff = open_tiff(filename).await;
        let ifd = &tiff.ifds()[0];
        assert_eq!(ifd.image_height(), 64);
        assert_eq!(ifd.image_width(), 64);
        assert_eq!(
            ifd.photometric_interpretation(),
            PhotometricInterpretation::RGB
        );
        assert!(ifd.bits_per_sample().iter().all(|x| *x == 8));
        assert_eq!(
            ifd.strip_offsets().expect("Cannot get StripOffsets"),
            vec![16]
        );
        assert_eq!(ifd.rows_per_strip().expect("Cannot get RowsPerStrip"), 64);
        assert_eq!(
            ifd.strip_byte_counts().expect("Cannot get StripByteCounts"),
            vec![12288]
        );
    }
}
