use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock};

use crate::geo::GeoKeyDirectory;
use crate::tags::Tag;
use crate::{tags, TagValue};

/// A registry for extensions that extend the set of tags able to be parsed from the TIFF
/// [`ImageFileDirectory``].
#[derive(Debug)]
pub struct ExtensionRegistry(HashMap<String, Box<dyn TiffExtensionFactory>>);

impl ExtensionRegistry {
    /// Create a new extension registry with no extensions registered
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn add(&mut self, extension: Box<dyn TiffExtensionFactory>) {
        // TODO: assert that no two extensions register the same tag values?
        // Or, you could allow multiple extensions to register the same tag, and call all
        // known extensions for every tag id they register.
        self.0.insert(extension.name().to_string(), extension);
    }

    pub(crate) fn inner(&self) -> &HashMap<String, Box<dyn TiffExtensionFactory>> {
        &self.0
    }
}

/// Something that knows how to create a TIFF extension.
pub trait TiffExtensionFactory: std::fmt::Debug + Send + Sync {
    /// The name of the extension.
    fn name(&self) -> &str;

    fn from_tags(&self, tag_data: HashMap<Tag, TagValue>) -> Box<dyn TiffExtension>;
}

/// Something that holds parsed IFD extension data.
pub trait TiffExtension: std::fmt::Debug + Send + Sync {
    /// The name of the extension.
    fn name(&self) -> &str;

    /// The u16 tag values this extension supports.
    fn supported_tags(&self) -> &HashSet<u16>;

    fn insert(&mut self, tag: u16, value: TagValue);

    fn finish(&mut self);

    fn parse_tag(&self, tag: u16) -> Option<Self::Tags>;
}

pub trait IfdExtension: std::fmt::Debug + Send + Sync {}

#[derive(Debug)]
pub struct GeoTIFFExtensionFactory;

impl TiffExtensionFactory for GeoTIFFExtensionFactory {
    fn name(&self) -> &str {
        "GeoTIFF"
    }

    fn from_tags(&self, tag_data: HashMap<Tag, TagValue>) -> Box<dyn TiffExtension> {
        let mut geo_key_directory_data = None;
        let mut model_pixel_scale = None;
        let mut model_tiepoint = None;
        let mut model_transformation = None;
        let mut geo_ascii_params: Option<String> = None;
        let mut geo_double_params: Option<Vec<f64>> = None;
        let mut gdal_nodata = None;
        let mut gdal_metadata = None;

        for (k, v) in tag_data {}
    }
}

#[derive(Debug)]
pub struct GeoTIFFExtension {
    // Geospatial tags
    pub(crate) geo_key_directory: Option<GeoKeyDirectory>,
    pub(crate) model_pixel_scale: Option<Vec<f64>>,
    pub(crate) model_tiepoint: Option<Vec<f64>>,
    pub(crate) model_transformation: Option<Vec<f64>>,

    // GDAL tags
    pub(crate) gdal_nodata: Option<String>,
    pub(crate) gdal_metadata: Option<String>,
}

tags! {
enum GeoTIFFTag(u16) {
    ModelPixelScale = 33550,
    ModelTransformation = 34264,
    ModelTiepoint = 33922,
    GeoKeyDirectory = 34735,
    GeoDoubleParams = 34736,
    GeoAsciiParams = 34737,
    GdalNodata = 42113,
    GdalMetadata = 42112,
}
}

static GEOTIFF_TAGS: LazyLock<HashSet<u16>> = LazyLock::new(|| {
    [
        GeoTIFFTag::ModelPixelScale,
        GeoTIFFTag::ModelTransformation,
        GeoTIFFTag::ModelTiepoint,
        GeoTIFFTag::GeoKeyDirectory,
        GeoTIFFTag::GeoDoubleParams,
        GeoTIFFTag::GeoAsciiParams,
        GeoTIFFTag::GdalNodata,
        GeoTIFFTag::GdalMetadata,
    ]
    .map(|x| x.to_u16())
    .iter()
    .copied()
    .collect()
});

impl TiffExtension for GeoTIFFExtension {
    fn name(&self) -> &str {
        "GeoTIFF"
    }

    fn supported_tags(&self) -> &HashSet<u16> {
        &GEOTIFF_TAGS
    }

    fn insert(&mut self, tag: u16, value: TagValue) {
        let geotiff_tag =
            GeoTIFFTag::from_u16(tag).expect("tag should be supported by this extension");

        match geotiff_tag {
            // Geospatial tags
            // http://geotiff.maptools.org/spec/geotiff2.4.html
            GeoTIFFTag::GeoKeyDirectory => {
                self.geo_key_directory_data = Some(value.into_u16_vec()?)
            }
            GeoTIFFTag::ModelPixelScale => model_pixel_scale = Some(value.into_f64_vec()?),
            GeoTIFFTag::ModelTiepoint => model_tiepoint = Some(value.into_f64_vec()?),
            GeoTIFFTag::ModelTransformation => model_transformation = Some(value.into_f64_vec()?),
            GeoTIFFTag::GeoAsciiParams => geo_ascii_params = Some(value.into_string()?),
            GeoTIFFTag::GeoDoubleParams => geo_double_params = Some(value.into_f64_vec()?),
            GeoTIFFTag::GdalNodata => gdal_nodata = Some(value.into_string()?),
            GeoTIFFTag::GdalMetadata => gdal_metadata = Some(value.into_string()?),
        }
    }
}
