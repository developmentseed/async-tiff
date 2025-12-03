//! Integration tests on OME-TIFF files.

use std::sync::Arc;

use async_tiff::metadata::cache::ReadaheadMetadataCache;
use async_tiff::metadata::TiffMetadataReader;
use async_tiff::reader::{AsyncFileReader, ObjectReader};
use async_tiff::tags::PhotometricInterpretation;
use async_tiff::TIFF;
use reqwest::Url;

async fn open_remote_tiff(url: &str) -> TIFF {
    let parsed_url = Url::parse(url).expect("failed parsing url");
    let (store, path) = object_store::parse_url(&parsed_url).unwrap();

    let reader = Arc::new(ObjectReader::new(Arc::new(store), path)) as Arc<dyn AsyncFileReader>;
    let cached_reader = ReadaheadMetadataCache::new(reader.clone());
    let mut metadata_reader = TiffMetadataReader::try_open(&cached_reader).await.unwrap();
    metadata_reader.read(&cached_reader).await.unwrap()
}

#[tokio::test]
async fn test_ome_tiff_single_channel() {
    let tiff =
        open_remote_tiff("https://downloads.openmicroscopy.org/images/OME-TIFF/2016-06/bioformats-artificial/single-channel.ome.tif").await;

    assert_eq!(tiff.ifds().len(), 1);
    let ifd = &tiff.ifds()[0];

    assert_eq!(
        ifd.photometric_interpretation(),
        PhotometricInterpretation::BlackIsZero
    );
    assert_eq!(ifd.image_description(), Some("<?xml version=\"1.0\" encoding=\"UTF-8\"?><!-- Warning: this comment is an OME-XML metadata block, which contains crucial dimensional parameters and other important metadata. Please edit cautiously (if at all), and back up the original data before doing so. For more information, see the OME-TIFF web site: http://www.openmicroscopy.org/site/support/ome-model/ome-tiff/. --><OME xmlns=\"http://www.openmicroscopy.org/Schemas/OME/2016-06\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" Creator=\"OME Bio-Formats 5.2.2\" UUID=\"urn:uuid:2bc2aa39-30d2-44ee-8399-c513492dd5de\" xsi:schemaLocation=\"http://www.openmicroscopy.org/Schemas/OME/2016-06 http://www.openmicroscopy.org/Schemas/OME/2016-06/ome.xsd\"><Image ID=\"Image:0\" Name=\"single-channel.ome.tif\"><Pixels BigEndian=\"true\" DimensionOrder=\"XYZCT\" ID=\"Pixels:0\" SizeC=\"1\" SizeT=\"1\" SizeX=\"439\" SizeY=\"167\" SizeZ=\"1\" Type=\"int8\"><Channel ID=\"Channel:0:0\" SamplesPerPixel=\"1\"><LightPath/></Channel><TiffData FirstC=\"0\" FirstT=\"0\" FirstZ=\"0\" IFD=\"0\" PlaneCount=\"1\"><UUID FileName=\"single-channel.ome.tif\">urn:uuid:2bc2aa39-30d2-44ee-8399-c513492dd5de</UUID></TiffData></Pixels></Image></OME>"));

    assert!(ifd.bits_per_sample().iter().all(|x| *x == 8));
}
