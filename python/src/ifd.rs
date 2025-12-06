use std::collections::HashMap;

use async_tiff::ImageFileDirectory;
use pyo3::prelude::*;
use pyo3::IntoPyObjectExt;

use crate::enums::{
    PyCompressionMethod, PyPhotometricInterpretation, PyPlanarConfiguration, PyPredictor,
    PyResolutionUnit, PySampleFormat,
};
use crate::geo::PyGeoKeyDirectory;
use crate::value::PyValue;

#[pyclass(name = "ImageFileDirectory", frozen, eq)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PyImageFileDirectory(ImageFileDirectory);

#[pymethods]
impl PyImageFileDirectory {
    #[getter]
    pub fn new_subfile_type(&self) -> Option<u32> {
        self.0.new_subfile_type()
    }

    /// The number of columns in the image, i.e., the number of pixels per row.
    #[getter]
    pub fn image_width(&self) -> u32 {
        self.0.image_width()
    }

    /// The number of rows of pixels in the image.
    #[getter]
    pub fn image_height(&self) -> u32 {
        self.0.image_height()
    }

    #[getter]
    pub fn bits_per_sample(&self) -> &[u16] {
        self.0.bits_per_sample()
    }

    #[getter]
    pub fn compression(&self) -> PyCompressionMethod {
        self.0.compression().into()
    }

    #[getter]
    pub fn photometric_interpretation(&self) -> PyPhotometricInterpretation {
        self.0.photometric_interpretation().into()
    }

    #[getter]
    pub fn document_name(&self) -> Option<&str> {
        self.0.document_name()
    }

    #[getter]
    pub fn image_description(&self) -> Option<&str> {
        self.0.image_description()
    }

    #[getter]
    pub fn strip_offsets(&self) -> Option<&[u64]> {
        self.0.strip_offsets()
    }

    #[getter]
    pub fn orientation(&self) -> Option<u16> {
        self.0.orientation()
    }

    /// The number of components per pixel.
    ///
    /// SamplesPerPixel is usually 1 for bilevel, grayscale, and palette-color images.
    /// SamplesPerPixel is usually 3 for RGB images. If this value is higher, ExtraSamples should
    /// give an indication of the meaning of the additional channels.
    #[getter]
    pub fn samples_per_pixel(&self) -> u16 {
        self.0.samples_per_pixel()
    }

    #[getter]
    pub fn rows_per_strip(&self) -> Option<u32> {
        self.0.rows_per_strip()
    }

    #[getter]
    pub fn strip_byte_counts(&self) -> Option<&[u64]> {
        self.0.strip_byte_counts()
    }

    #[getter]
    pub fn min_sample_value(&self) -> Option<&[u16]> {
        self.0.min_sample_value()
    }

    #[getter]
    pub fn max_sample_value(&self) -> Option<&[u16]> {
        self.0.max_sample_value()
    }

    /// The number of pixels per ResolutionUnit in the ImageWidth direction.
    #[getter]
    pub fn x_resolution(&self) -> Option<f64> {
        self.0.x_resolution()
    }

    /// The number of pixels per ResolutionUnit in the ImageLength direction.
    #[getter]
    pub fn y_resolution(&self) -> Option<f64> {
        self.0.y_resolution()
    }

    /// How the components of each pixel are stored.
    ///
    /// The specification defines these values:
    ///
    /// - Chunky format. The component values for each pixel are stored contiguously. For example,
    ///   for RGB data, the data is stored as RGBRGBRGB
    /// - Planar format. The components are stored in separate component planes. For example, RGB
    ///   data is stored with the Red components in one component plane, the Green in another, and
    ///   the Blue in another.
    ///
    /// The specification adds a warning that PlanarConfiguration=2 is not in widespread use and
    /// that Baseline TIFF readers are not required to support it.
    ///
    /// If SamplesPerPixel is 1, PlanarConfiguration is irrelevant, and need not be included.
    #[getter]
    pub fn planar_configuration(&self) -> PyPlanarConfiguration {
        self.0.planar_configuration().into()
    }

    #[getter]
    pub fn resolution_unit(&self) -> Option<PyResolutionUnit> {
        self.0.resolution_unit().map(|x| x.into())
    }

    /// Name and version number of the software package(s) used to create the image.
    #[getter]
    pub fn software(&self) -> Option<&str> {
        self.0.software()
    }

    /// Date and time of image creation.
    ///
    /// The format is: "YYYY:MM:DD HH:MM:SS", with hours like those on a 24-hour clock, and one
    /// space character between the date and the time. The length of the string, including the
    /// terminating NUL, is 20 bytes.
    #[getter]
    pub fn date_time(&self) -> Option<&str> {
        self.0.date_time()
    }

    #[getter]
    pub fn artist(&self) -> Option<&str> {
        self.0.artist()
    }

    #[getter]
    pub fn host_computer(&self) -> Option<&str> {
        self.0.host_computer()
    }

    #[getter]
    pub fn predictor(&self) -> Option<PyPredictor> {
        self.0.predictor().map(|x| x.into())
    }

    #[getter]
    pub fn tile_width(&self) -> Option<u32> {
        self.0.tile_width()
    }
    #[getter]
    pub fn tile_height(&self) -> Option<u32> {
        self.0.tile_height()
    }

    #[getter]
    pub fn tile_offsets(&self) -> Option<&[u64]> {
        self.0.tile_offsets()
    }
    #[getter]
    pub fn tile_byte_counts(&self) -> Option<&[u64]> {
        self.0.tile_byte_counts()
    }

    #[getter]
    pub fn extra_samples(&self) -> Option<&[u16]> {
        self.0.extra_samples()
    }

    #[getter]
    pub fn sample_format(&self) -> Vec<PySampleFormat> {
        self.0.sample_format().iter().map(|x| (*x).into()).collect()
    }

    #[getter]
    pub fn jpeg_tables(&self) -> Option<&[u8]> {
        self.0.jpeg_tables()
    }

    #[getter]
    pub fn copyright(&self) -> Option<&str> {
        self.0.copyright()
    }

    // Geospatial tags
    #[getter]
    pub fn geo_key_directory(&self) -> Option<PyGeoKeyDirectory> {
        self.0.geo_key_directory().cloned().map(|x| x.into())
    }

    #[getter]
    pub fn model_pixel_scale(&self) -> Option<&[f64]> {
        self.0.model_pixel_scale()
    }

    #[getter]
    pub fn model_tiepoint(&self) -> Option<&[f64]> {
        self.0.model_tiepoint()
    }

    #[getter]
    pub fn other_tags(&self) -> HashMap<u16, PyValue> {
        let iter = self
            .0
            .other_tags()
            .iter()
            .map(|(key, val)| (key.to_u16(), val.clone().into()));
        HashMap::from_iter(iter)
    }

    /// This exists to implement the Mapping protocol, so we support `dict(ifd)`.`
    fn keys(&self) -> Vec<&'static str> {
        // Always present keys
        let mut keys = vec![
            "image_width",
            "image_height",
            "bits_per_sample",
            "compression",
            "photometric_interpretation",
            "samples_per_pixel",
            "planar_configuration",
            "sample_format",
            "other_tags",
        ];

        // Optional keys
        if self.new_subfile_type().is_some() {
            keys.push("new_subfile_type");
        }
        if self.document_name().is_some() {
            keys.push("document_name");
        }
        if self.image_description().is_some() {
            keys.push("image_description");
        }
        if self.strip_offsets().is_some() {
            keys.push("strip_offsets");
        }
        if self.orientation().is_some() {
            keys.push("orientation");
        }
        if self.rows_per_strip().is_some() {
            keys.push("rows_per_strip");
        }
        if self.strip_byte_counts().is_some() {
            keys.push("strip_byte_counts");
        }
        if self.min_sample_value().is_some() {
            keys.push("min_sample_value");
        }
        if self.max_sample_value().is_some() {
            keys.push("max_sample_value");
        }
        if self.x_resolution().is_some() {
            keys.push("x_resolution");
        }
        if self.y_resolution().is_some() {
            keys.push("y_resolution");
        }
        if self.resolution_unit().is_some() {
            keys.push("resolution_unit");
        }
        if self.software().is_some() {
            keys.push("software");
        }
        if self.date_time().is_some() {
            keys.push("date_time");
        }
        if self.artist().is_some() {
            keys.push("artist");
        }
        if self.host_computer().is_some() {
            keys.push("host_computer");
        }
        if self.predictor().is_some() {
            keys.push("predictor");
        }
        if self.tile_width().is_some() {
            keys.push("tile_width");
        }
        if self.tile_height().is_some() {
            keys.push("tile_height");
        }
        if self.tile_offsets().is_some() {
            keys.push("tile_offsets");
        }
        if self.tile_byte_counts().is_some() {
            keys.push("tile_byte_counts");
        }
        if self.extra_samples().is_some() {
            keys.push("extra_samples");
        }
        if self.jpeg_tables().is_some() {
            keys.push("jpeg_tables");
        }
        if self.copyright().is_some() {
            keys.push("copyright");
        }
        if self.geo_key_directory().is_some() {
            keys.push("geo_key_directory");
        }
        if self.model_pixel_scale().is_some() {
            keys.push("model_pixel_scale");
        }
        if self.model_tiepoint().is_some() {
            keys.push("model_tiepoint");
        }

        keys
    }

    /// This exists to implement the Mapping protocol, so we support `dict(ifd)`.`
    fn __iter__<'py>(&'py self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.keys().into_pyobject(py)?.call_method0("__iter__")
    }

    /// Get an IFD property by name
    /// This exists to implement the Mapping protocol, so we support `dict(ifd)`.`
    fn __getitem__<'py>(&self, py: Python<'py>, key: &str) -> PyResult<Bound<'py, PyAny>> {
        match key {
            "new_subfile_type" => self.new_subfile_type().into_bound_py_any(py),
            "image_width" => self.image_width().into_bound_py_any(py),
            "image_height" => self.image_height().into_bound_py_any(py),
            "bits_per_sample" => self.bits_per_sample().into_bound_py_any(py),
            "compression" => self.compression().into_bound_py_any(py),
            "photometric_interpretation" => self.photometric_interpretation().into_bound_py_any(py),
            "document_name" => self.document_name().into_bound_py_any(py),
            "image_description" => self.image_description().into_bound_py_any(py),
            "strip_offsets" => self.strip_offsets().into_bound_py_any(py),
            "orientation" => self.orientation().into_bound_py_any(py),
            "samples_per_pixel" => self.samples_per_pixel().into_bound_py_any(py),
            "rows_per_strip" => self.rows_per_strip().into_bound_py_any(py),
            "strip_byte_counts" => self.strip_byte_counts().into_bound_py_any(py),
            "min_sample_value" => self.min_sample_value().into_bound_py_any(py),
            "max_sample_value" => self.max_sample_value().into_bound_py_any(py),
            "x_resolution" => self.x_resolution().into_bound_py_any(py),
            "y_resolution" => self.y_resolution().into_bound_py_any(py),
            "planar_configuration" => self.planar_configuration().into_bound_py_any(py),
            "resolution_unit" => self.resolution_unit().into_bound_py_any(py),
            "software" => self.software().into_bound_py_any(py),
            "date_time" => self.date_time().into_bound_py_any(py),
            "artist" => self.artist().into_bound_py_any(py),
            "host_computer" => self.host_computer().into_bound_py_any(py),
            "predictor" => self.predictor().into_bound_py_any(py),
            "tile_width" => self.tile_width().into_bound_py_any(py),
            "tile_height" => self.tile_height().into_bound_py_any(py),
            "tile_offsets" => self.tile_offsets().into_bound_py_any(py),
            "tile_byte_counts" => self.tile_byte_counts().into_bound_py_any(py),
            "extra_samples" => self.extra_samples().into_bound_py_any(py),
            "sample_format" => self.sample_format().into_bound_py_any(py),
            "jpeg_tables" => self.jpeg_tables().into_bound_py_any(py),
            "copyright" => self.copyright().into_bound_py_any(py),
            "geo_key_directory" => self.geo_key_directory().into_bound_py_any(py),
            "model_pixel_scale" => self.model_pixel_scale().into_bound_py_any(py),
            "model_tiepoint" => self.model_tiepoint().into_bound_py_any(py),
            "other_tags" => self.other_tags().into_bound_py_any(py),
            _ => Err(pyo3::exceptions::PyKeyError::new_err(format!(
                "Unknown IFD property: {}",
                key
            ))),
        }
    }
}

impl From<ImageFileDirectory> for PyImageFileDirectory {
    fn from(value: ImageFileDirectory) -> Self {
        Self(value)
    }
}
