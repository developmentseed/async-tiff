use async_tiff::geo::GeoKeyDirectory;
use pyo3::prelude::*;
use pyo3::IntoPyObjectExt;

#[pyclass(name = "GeoKeyDirectory", frozen, eq, get_all)]
#[derive(PartialEq)]
pub(crate) struct PyGeoKeyDirectory {
    model_type: Option<u16>,
    raster_type: Option<u16>,
    citation: Option<String>,
    geographic_type: Option<u16>,
    geog_citation: Option<String>,
    geog_geodetic_datum: Option<u16>,
    geog_prime_meridian: Option<u16>,
    geog_linear_units: Option<u16>,
    geog_linear_unit_size: Option<f64>,
    geog_angular_units: Option<u16>,
    geog_angular_unit_size: Option<f64>,
    geog_ellipsoid: Option<u16>,
    geog_semi_major_axis: Option<f64>,
    geog_semi_minor_axis: Option<f64>,
    geog_inv_flattening: Option<f64>,
    geog_azimuth_units: Option<u16>,
    geog_prime_meridian_long: Option<f64>,

    projected_type: Option<u16>,
    proj_citation: Option<String>,
    projection: Option<u16>,
    proj_coord_trans: Option<u16>,
    proj_linear_units: Option<u16>,
    proj_linear_unit_size: Option<f64>,
    proj_std_parallel1: Option<f64>,
    proj_std_parallel2: Option<f64>,
    proj_nat_origin_long: Option<f64>,
    proj_nat_origin_lat: Option<f64>,
    proj_false_easting: Option<f64>,
    proj_false_northing: Option<f64>,
    proj_false_origin_long: Option<f64>,
    proj_false_origin_lat: Option<f64>,
    proj_false_origin_easting: Option<f64>,
    proj_false_origin_northing: Option<f64>,
    proj_center_long: Option<f64>,
    proj_center_lat: Option<f64>,
    proj_center_easting: Option<f64>,
    proj_center_northing: Option<f64>,
    proj_scale_at_nat_origin: Option<f64>,
    proj_scale_at_center: Option<f64>,
    proj_azimuth_angle: Option<f64>,
    proj_straight_vert_pole_long: Option<f64>,

    vertical: Option<u16>,
    vertical_citation: Option<String>,
    vertical_datum: Option<u16>,
    vertical_units: Option<u16>,
}

#[pymethods]
impl PyGeoKeyDirectory {
    /// This exists to implement the Mapping protocol, so we support `dict(gkd)`.`
    fn keys(&self) -> Vec<&'static str> {
        let mut keys = vec![];
        if self.model_type.is_some() {
            keys.push("model_type");
        }
        if self.raster_type.is_some() {
            keys.push("raster_type");
        }
        if self.citation.is_some() {
            keys.push("citation");
        }
        if self.geographic_type.is_some() {
            keys.push("geographic_type");
        }
        if self.geog_citation.is_some() {
            keys.push("geog_citation");
        }
        if self.geog_geodetic_datum.is_some() {
            keys.push("geog_geodetic_datum");
        }
        if self.geog_prime_meridian.is_some() {
            keys.push("geog_prime_meridian");
        }
        if self.geog_linear_units.is_some() {
            keys.push("geog_linear_units");
        }
        if self.geog_linear_unit_size.is_some() {
            keys.push("geog_linear_unit_size");
        }
        if self.geog_angular_units.is_some() {
            keys.push("geog_angular_units");
        }
        if self.geog_angular_unit_size.is_some() {
            keys.push("geog_angular_unit_size");
        }
        if self.geog_ellipsoid.is_some() {
            keys.push("geog_ellipsoid");
        }
        if self.geog_semi_major_axis.is_some() {
            keys.push("geog_semi_major_axis");
        }
        if self.geog_semi_minor_axis.is_some() {
            keys.push("geog_semi_minor_axis");
        }
        if self.geog_inv_flattening.is_some() {
            keys.push("geog_inv_flattening");
        }
        if self.geog_azimuth_units.is_some() {
            keys.push("geog_azimuth_units");
        }
        if self.geog_prime_meridian_long.is_some() {
            keys.push("geog_prime_meridian_long");
        }
        if self.projected_type.is_some() {
            keys.push("projected_type");
        }
        if self.proj_citation.is_some() {
            keys.push("proj_citation");
        }
        if self.projection.is_some() {
            keys.push("projection");
        }
        if self.proj_coord_trans.is_some() {
            keys.push("proj_coord_trans");
        }
        if self.proj_linear_units.is_some() {
            keys.push("proj_linear_units");
        }
        if self.proj_linear_unit_size.is_some() {
            keys.push("proj_linear_unit_size");
        }
        if self.proj_std_parallel1.is_some() {
            keys.push("proj_std_parallel1");
        }
        if self.proj_std_parallel2.is_some() {
            keys.push("proj_std_parallel2");
        }
        if self.proj_nat_origin_long.is_some() {
            keys.push("proj_nat_origin_long");
        }
        if self.proj_nat_origin_lat.is_some() {
            keys.push("proj_nat_origin_lat");
        }
        if self.proj_false_easting.is_some() {
            keys.push("proj_false_easting");
        }
        if self.proj_false_northing.is_some() {
            keys.push("proj_false_northing");
        }
        if self.proj_false_origin_long.is_some() {
            keys.push("proj_false_origin_long");
        }
        if self.proj_false_origin_lat.is_some() {
            keys.push("proj_false_origin_lat");
        }
        if self.proj_false_origin_easting.is_some() {
            keys.push("proj_false_origin_easting");
        }
        if self.proj_false_origin_northing.is_some() {
            keys.push("proj_false_origin_northing");
        }
        if self.proj_center_long.is_some() {
            keys.push("proj_center_long");
        }
        if self.proj_center_lat.is_some() {
            keys.push("proj_center_lat");
        }
        if self.proj_center_easting.is_some() {
            keys.push("proj_center_easting");
        }
        if self.proj_center_northing.is_some() {
            keys.push("proj_center_northing");
        }
        if self.proj_scale_at_nat_origin.is_some() {
            keys.push("proj_scale_at_nat_origin");
        }
        if self.proj_scale_at_center.is_some() {
            keys.push("proj_scale_at_center");
        }
        if self.proj_azimuth_angle.is_some() {
            keys.push("proj_azimuth_angle");
        }
        if self.proj_straight_vert_pole_long.is_some() {
            keys.push("proj_straight_vert_pole_long");
        }
        if self.vertical.is_some() {
            keys.push("vertical");
        }
        if self.vertical_citation.is_some() {
            keys.push("vertical_citation");
        }
        if self.vertical_datum.is_some() {
            keys.push("vertical_datum");
        }
        if self.vertical_units.is_some() {
            keys.push("vertical_units");
        }

        keys
    }

    /// This exists to implement the Mapping protocol, so we support `dict(gkd)`.`
    fn __iter__<'py>(&'py self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.keys().into_pyobject(py)?.call_method0("__iter__")
    }

    /// Get a GeoKeyDirectory property by name
    /// This exists to implement the Mapping protocol, so we support `dict(gkd)`.`
    fn __getitem__<'py>(&self, py: Python<'py>, key: &str) -> PyResult<Bound<'py, PyAny>> {
        match key {
            "model_type" => self.model_type.into_bound_py_any(py),
            "raster_type" => self.raster_type.into_bound_py_any(py),
            "citation" => self.citation.as_ref().into_bound_py_any(py),
            "geographic_type" => self.geographic_type.into_bound_py_any(py),
            "geog_citation" => self.geog_citation.as_ref().into_bound_py_any(py),
            "geog_geodetic_datum" => self.geog_geodetic_datum.into_bound_py_any(py),
            "geog_prime_meridian" => self.geog_prime_meridian.into_bound_py_any(py),
            "geog_linear_units" => self.geog_linear_units.into_bound_py_any(py),
            "geog_linear_unit_size" => self.geog_linear_unit_size.into_bound_py_any(py),
            "geog_angular_units" => self.geog_angular_units.into_bound_py_any(py),
            "geog_angular_unit_size" => self.geog_angular_unit_size.into_bound_py_any(py),
            "geog_ellipsoid" => self.geog_ellipsoid.into_bound_py_any(py),
            "geog_semi_major_axis" => self.geog_semi_major_axis.into_bound_py_any(py),
            "geog_semi_minor_axis" => self.geog_semi_minor_axis.into_bound_py_any(py),
            "geog_inv_flattening" => self.geog_inv_flattening.into_bound_py_any(py),
            "geog_azimuth_units" => self.geog_azimuth_units.into_bound_py_any(py),
            "geog_prime_meridian_long" => self.geog_prime_meridian_long.into_bound_py_any(py),
            "projected_type" => self.projected_type.into_bound_py_any(py),
            "proj_citation" => self.proj_citation.as_ref().into_bound_py_any(py),
            "projection" => self.projection.into_bound_py_any(py),
            "proj_coord_trans" => self.proj_coord_trans.into_bound_py_any(py),
            "proj_linear_units" => self.proj_linear_units.into_bound_py_any(py),
            "proj_linear_unit_size" => self.proj_linear_unit_size.into_bound_py_any(py),
            "proj_std_parallel1" => self.proj_std_parallel1.into_bound_py_any(py),
            "proj_std_parallel2" => self.proj_std_parallel2.into_bound_py_any(py),
            "proj_nat_origin_long" => self.proj_nat_origin_long.into_bound_py_any(py),
            "proj_nat_origin_lat" => self.proj_nat_origin_lat.into_bound_py_any(py),
            "proj_false_easting" => self.proj_false_easting.into_bound_py_any(py),
            "proj_false_northing" => self.proj_false_northing.into_bound_py_any(py),
            "proj_false_origin_long" => self.proj_false_origin_long.into_bound_py_any(py),
            "proj_false_origin_lat" => self.proj_false_origin_lat.into_bound_py_any(py),
            "proj_false_origin_easting" => self.proj_false_origin_easting.into_bound_py_any(py),
            "proj_false_origin_northing" => self.proj_false_origin_northing.into_bound_py_any(py),
            "proj_center_long" => self.proj_center_long.into_bound_py_any(py),
            "proj_center_lat" => self.proj_center_lat.into_bound_py_any(py),
            "proj_center_easting" => self.proj_center_easting.into_bound_py_any(py),
            "proj_center_northing" => self.proj_center_northing.into_bound_py_any(py),
            "proj_scale_at_nat_origin" => self.proj_scale_at_nat_origin.into_bound_py_any(py),
            "proj_scale_at_center" => self.proj_scale_at_center.into_bound_py_any(py),
            "proj_azimuth_angle" => self.proj_azimuth_angle.into_bound_py_any(py),
            "proj_straight_vert_pole_long" => {
                self.proj_straight_vert_pole_long.into_bound_py_any(py)
            }
            "vertical" => self.vertical.into_bound_py_any(py),
            "vertical_citation" => self.vertical_citation.as_ref().into_bound_py_any(py),
            "vertical_datum" => self.vertical_datum.into_bound_py_any(py),
            "vertical_units" => self.vertical_units.into_bound_py_any(py),
            _ => Err(pyo3::exceptions::PyKeyError::new_err(format!(
                "Unknown IFD property: {}",
                key
            ))),
        }
    }
}

impl From<PyGeoKeyDirectory> for GeoKeyDirectory {
    fn from(value: PyGeoKeyDirectory) -> Self {
        Self {
            model_type: value.model_type,
            raster_type: value.raster_type,
            citation: value.citation,
            geographic_type: value.geographic_type,
            geog_citation: value.geog_citation,
            geog_geodetic_datum: value.geog_geodetic_datum,
            geog_prime_meridian: value.geog_prime_meridian,
            geog_linear_units: value.geog_linear_units,
            geog_linear_unit_size: value.geog_linear_unit_size,
            geog_angular_units: value.geog_angular_units,
            geog_angular_unit_size: value.geog_angular_unit_size,
            geog_ellipsoid: value.geog_ellipsoid,
            geog_semi_major_axis: value.geog_semi_major_axis,
            geog_semi_minor_axis: value.geog_semi_minor_axis,
            geog_inv_flattening: value.geog_inv_flattening,
            geog_azimuth_units: value.geog_azimuth_units,
            geog_prime_meridian_long: value.geog_prime_meridian_long,
            projected_type: value.projected_type,
            proj_citation: value.proj_citation,
            projection: value.projection,
            proj_coord_trans: value.proj_coord_trans,
            proj_linear_units: value.proj_linear_units,
            proj_linear_unit_size: value.proj_linear_unit_size,
            proj_std_parallel1: value.proj_std_parallel1,
            proj_std_parallel2: value.proj_std_parallel2,
            proj_nat_origin_long: value.proj_nat_origin_long,
            proj_nat_origin_lat: value.proj_nat_origin_lat,
            proj_false_easting: value.proj_false_easting,
            proj_false_northing: value.proj_false_northing,
            proj_false_origin_long: value.proj_false_origin_long,
            proj_false_origin_lat: value.proj_false_origin_lat,
            proj_false_origin_easting: value.proj_false_origin_easting,
            proj_false_origin_northing: value.proj_false_origin_northing,
            proj_center_long: value.proj_center_long,
            proj_center_lat: value.proj_center_lat,
            proj_center_easting: value.proj_center_easting,
            proj_center_northing: value.proj_center_northing,
            proj_scale_at_nat_origin: value.proj_scale_at_nat_origin,
            proj_scale_at_center: value.proj_scale_at_center,
            proj_azimuth_angle: value.proj_azimuth_angle,
            proj_straight_vert_pole_long: value.proj_straight_vert_pole_long,
            vertical: value.vertical,
            vertical_citation: value.vertical_citation,
            vertical_datum: value.vertical_datum,
            vertical_units: value.vertical_units,
        }
    }
}

impl From<GeoKeyDirectory> for PyGeoKeyDirectory {
    fn from(value: GeoKeyDirectory) -> Self {
        Self {
            model_type: value.model_type,
            raster_type: value.raster_type,
            citation: value.citation,
            geographic_type: value.geographic_type,
            geog_citation: value.geog_citation,
            geog_geodetic_datum: value.geog_geodetic_datum,
            geog_prime_meridian: value.geog_prime_meridian,
            geog_linear_units: value.geog_linear_units,
            geog_linear_unit_size: value.geog_linear_unit_size,
            geog_angular_units: value.geog_angular_units,
            geog_angular_unit_size: value.geog_angular_unit_size,
            geog_ellipsoid: value.geog_ellipsoid,
            geog_semi_major_axis: value.geog_semi_major_axis,
            geog_semi_minor_axis: value.geog_semi_minor_axis,
            geog_inv_flattening: value.geog_inv_flattening,
            geog_azimuth_units: value.geog_azimuth_units,
            geog_prime_meridian_long: value.geog_prime_meridian_long,
            projected_type: value.projected_type,
            proj_citation: value.proj_citation,
            projection: value.projection,
            proj_coord_trans: value.proj_coord_trans,
            proj_linear_units: value.proj_linear_units,
            proj_linear_unit_size: value.proj_linear_unit_size,
            proj_std_parallel1: value.proj_std_parallel1,
            proj_std_parallel2: value.proj_std_parallel2,
            proj_nat_origin_long: value.proj_nat_origin_long,
            proj_nat_origin_lat: value.proj_nat_origin_lat,
            proj_false_easting: value.proj_false_easting,
            proj_false_northing: value.proj_false_northing,
            proj_false_origin_long: value.proj_false_origin_long,
            proj_false_origin_lat: value.proj_false_origin_lat,
            proj_false_origin_easting: value.proj_false_origin_easting,
            proj_false_origin_northing: value.proj_false_origin_northing,
            proj_center_long: value.proj_center_long,
            proj_center_lat: value.proj_center_lat,
            proj_center_easting: value.proj_center_easting,
            proj_center_northing: value.proj_center_northing,
            proj_scale_at_nat_origin: value.proj_scale_at_nat_origin,
            proj_scale_at_center: value.proj_scale_at_center,
            proj_azimuth_angle: value.proj_azimuth_angle,
            proj_straight_vert_pole_long: value.proj_straight_vert_pole_long,
            vertical: value.vertical,
            vertical_citation: value.vertical_citation,
            vertical_datum: value.vertical_datum,
            vertical_units: value.vertical_units,
        }
    }
}
