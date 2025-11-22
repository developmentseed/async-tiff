//! Integration tests on OME-TIFF files.

use std::sync::Arc;

use async_tiff::metadata::{PrefetchBuffer, TiffMetadataReader};
use async_tiff::reader::{AsyncFileReader, ObjectReader};
use async_tiff::tiff::tags::PhotometricInterpretation;
use async_tiff::TIFF;
use reqwest::Url;

async fn open_remote_tiff(url: &str, prefetch_bytes: u64) -> TIFF {
    let parsed_url = Url::parse(url).expect("failed parsing url");
    let (store, path) = object_store::parse_url(&parsed_url).unwrap();

    let reader = Arc::new(ObjectReader::new(Arc::new(store), path)) as Arc<dyn AsyncFileReader>;
    let prefetch_reader = PrefetchBuffer::new(reader.clone(), prefetch_bytes)
        .await
        .unwrap();
    let mut metadata_reader = TiffMetadataReader::try_open(&prefetch_reader)
        .await
        .unwrap();
    let ifds = metadata_reader
        .read_all_ifds(&prefetch_reader)
        .await
        .unwrap();
    TIFF::new(ifds)
}

#[tokio::test]
async fn test_ome_tiff_single_channel() {
    let tiff = open_remote_tiff(
        "https://cildata.crbs.ucsd.edu/media/images/40613/40613.tif",
        32 * 128 * 1024,
    )
    .await;

    assert_eq!(tiff.ifds().len(), 3);
    let ifd = &tiff.ifds()[0];

    assert_eq!(
        ifd.photometric_interpretation(),
        PhotometricInterpretation::BlackIsZero
    );
    assert_eq!(
        ifd.image_description(),
        Some(
            r##"<?xml version="1.0" encoding="UTF-8"?><!-- Warning: this comment is an OME-XML metadata block, which contains crucial dimensional parameters and other important metadata. Please edit cautiously (if at all), and back up the original data before doing so. For more information, see the OME-TIFF web site: http://loci.wisc.edu/ome/ome-tiff.html. -->
<OME xmlns:AML="http://www.openmicroscopy.org/Schemas/AnalysisModule/2008-09" xmlns:Bin="http://www.openmicroscopy.org/Schemas/BinaryFile/2008-09" xmlns:MLI="http://www.openmicroscopy.org/Schemas/MLI/2008-09" xmlns:SA="http://www.openmicroscopy.org/Schemas/SA/2008-09" xmlns:OME="http://www.openmicroscopy.org/Schemas/OME/2008-09" xmlns:SPW="http://www.openmicroscopy.org/Schemas/SPW/2008-09" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:CA="http://www.openmicroscopy.org/Schemas/CA/2008-09" xmlns:STD="http://www.openmicroscopy.org/Schemas/STD/2008-09" xmlns:CLI="http://www.openmicroscopy.org/Schemas/CLI/2008-09" xmlns="http://www.openmicroscopy.org/Schemas/OME/2008-09" xsi:schemaLocation="http://www.openmicroscopy.org/Schemas/OME/2008-09 http://www.openmicroscopy.org/Schemas/OME/2008-09/ome.xsd http://www.openmicroscopy.org/Schemas/BinaryFile/2008-09 http://www.openmicroscopy.org/Schemas/BinaryFile/2008-09/BinaryFile.xsd http://www.openmicroscopy.org/Schemas/CA/2008-09 http://www.openmicroscopy.org/Schemas/CA/2008-09/CA.xsd http://www.openmicroscopy.org/Schemas/SPW/2008-09 http://www.openmicroscopy.org/Schemas/SPW/2008-09/SPW.xsd http://www.openmicroscopy.org/Schemas/STD/2008-09 http://www.openmicroscopy.org/Schemas/STD/2008-09/STD.xsd" UUID="3af39f55-0ac0-431a-bc60-8f9c3e782b85">
<Experimenter ID="urn:lsid:export.openmicroscopy.org:Experimenter:46be26c5-9fcf-497e-a913-48513759f00b_3"/>
<Group ID="urn:lsid:export.openmicroscopy.org:ExperimenterGroup:46be26c5-9fcf-497e-a913-48513759f00b_54:3259518"/>
<Image DefaultPixels="urn:lsid:export.openmicroscopy.org:Pixels:46be26c5-9fcf-497e-a913-48513759f00b_40613:24177120" ID="urn:lsid:export.openmicroscopy.org:Image:46be26c5-9fcf-497e-a913-48513759f00b_40613:25467732" Name="IM_20100715_Nikitina_3_Niki_6_004_001_2_����������_lg.jpg">
<CreationDate>2012-03-25 21:26:29.0</CreationDate>
<ExperimenterRef ID="urn:lsid:export.openmicroscopy.org:Experimenter:46be26c5-9fcf-497e-a913-48513759f00b_3"/>
<GroupRef ID="urn:lsid:export.openmicroscopy.org:ExperimenterGroup:46be26c5-9fcf-497e-a913-48513759f00b_54:3259518"/>
<LogicalChannel ID="urn:lsid:export.openmicroscopy.org:LogicalChannel:46be26c5-9fcf-497e-a913-48513759f00b_81798:24177119" Name="Red" SamplesPerPixel="1"/>
<LogicalChannel ID="urn:lsid:export.openmicroscopy.org:LogicalChannel:46be26c5-9fcf-497e-a913-48513759f00b_81799:24177119" Name="Green" SamplesPerPixel="1"/>
<LogicalChannel ID="urn:lsid:export.openmicroscopy.org:LogicalChannel:46be26c5-9fcf-497e-a913-48513759f00b_81800:24177119" Name="Blue" SamplesPerPixel="1"/>
<Pixels BigEndian="true" DimensionOrder="XYCZT" ID="urn:lsid:export.openmicroscopy.org:Pixels:46be26c5-9fcf-497e-a913-48513759f00b_40613:24177120" PixelType="uint8" SizeC="3" SizeT="1" SizeX="1024" SizeY="943" SizeZ="1">
<TiffData>
<UUID FileName="__omero_export__2069104425008571311.ome.tiff">3af39f55-0ac0-431a-bc60-8f9c3e782b85</UUID>
</TiffData>
</Pixels>
</Image>
</OME>
"##
        )
    );

    assert!(ifd.bits_per_sample().iter().all(|x| *x == 8));
    assert_eq!(ifd.software(), Some("LOCI Bio-Formats"));
}
