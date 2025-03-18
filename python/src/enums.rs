use async_tiff::tiff::tags::{
    CompressionMethod, PhotometricInterpretation, PlanarConfiguration, Predictor, ResolutionUnit,
    SampleFormat,
};
use pyo3::prelude::*;
use pyo3::types::{PyString, PyTuple};
use pyo3::{intern, IntoPyObjectExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PyCompressionMethod(CompressionMethod);

impl From<CompressionMethod> for PyCompressionMethod {
    fn from(value: CompressionMethod) -> Self {
        Self(value)
    }
}

impl From<PyCompressionMethod> for CompressionMethod {
    fn from(value: PyCompressionMethod) -> Self {
        value.0
    }
}

impl<'py> FromPyObject<'py> for PyCompressionMethod {
    fn extract_bound(ob: &Bound<'py, PyAny>) -> PyResult<Self> {
        Ok(Self(CompressionMethod::from_u16_exhaustive(ob.extract()?)))
    }
}

impl<'py> IntoPyObject<'py> for PyCompressionMethod {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        to_py_enum_variant(py, intern!(py, "CompressionMethod"), self.0.to_u16())
    }
}

pub(crate) struct PyPhotometricInterpretation(PhotometricInterpretation);

impl From<PhotometricInterpretation> for PyPhotometricInterpretation {
    fn from(value: PhotometricInterpretation) -> Self {
        Self(value)
    }
}

impl<'py> IntoPyObject<'py> for PyPhotometricInterpretation {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        to_py_enum_variant(
            py,
            intern!(py, "PhotometricInterpretation"),
            self.0.to_u16(),
        )
    }
}

pub(crate) struct PyPlanarConfiguration(PlanarConfiguration);

impl From<PlanarConfiguration> for PyPlanarConfiguration {
    fn from(value: PlanarConfiguration) -> Self {
        Self(value)
    }
}

impl<'py> IntoPyObject<'py> for PyPlanarConfiguration {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        to_py_enum_variant(py, intern!(py, "PlanarConfiguration"), self.0.to_u16())
    }
}

pub(crate) struct PyResolutionUnit(ResolutionUnit);

impl From<ResolutionUnit> for PyResolutionUnit {
    fn from(value: ResolutionUnit) -> Self {
        Self(value)
    }
}

impl<'py> IntoPyObject<'py> for PyResolutionUnit {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        to_py_enum_variant(py, intern!(py, "ResolutionUnit"), self.0.to_u16())
    }
}

pub(crate) struct PyPredictor(Predictor);

impl From<Predictor> for PyPredictor {
    fn from(value: Predictor) -> Self {
        Self(value)
    }
}

impl<'py> IntoPyObject<'py> for PyPredictor {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        to_py_enum_variant(py, intern!(py, "Predictor"), self.0.to_u16())
    }
}

pub(crate) struct PySampleFormat(SampleFormat);

impl From<SampleFormat> for PySampleFormat {
    fn from(value: SampleFormat) -> Self {
        Self(value)
    }
}

impl<'py> IntoPyObject<'py> for PySampleFormat {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        to_py_enum_variant(py, intern!(py, "SampleFormat"), self.0.to_u16())
    }
}
fn to_py_enum_variant<'py>(
    py: Python<'py>,
    enum_name: &Bound<'py, PyString>,
    value: u16,
) -> PyResult<Bound<'py, PyAny>> {
    let enums_mod = py.import(intern!(py, "async_tiff.enums"))?;
    if let Ok(enum_variant) = enums_mod.call_method1(enum_name, PyTuple::new(py, vec![value])?) {
        Ok(enum_variant)
    } else {
        // If the value is not included in the enum, return the integer itself
        value.into_bound_py_any(py)
    }
}
