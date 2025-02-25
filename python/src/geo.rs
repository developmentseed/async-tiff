use async_tiff::geo::GeoKeyDirectory;
use pyo3::prelude::*;

#[pyclass(name = "GeoKeyDirectory")]
pub(crate) struct PyGeoKeyDirectory {
    #[pyo3(get)]
    model_type: Option<u16>,
    #[pyo3(get)]
    raster_type: Option<u16>,
    #[pyo3(get)]
    citation: Option<String>,
    #[pyo3(get)]
    geographic_type: Option<u16>,
    #[pyo3(get)]
    geog_citation: Option<String>,
    #[pyo3(get)]
    geog_geodetic_datum: Option<u16>,
    #[pyo3(get)]
    geog_prime_meridian: Option<u16>,
    #[pyo3(get)]
    geog_linear_units: Option<u16>,
    #[pyo3(get)]
    geog_linear_unit_size: Option<f64>,
    #[pyo3(get)]
    geog_angular_units: Option<u16>,
    #[pyo3(get)]
    geog_angular_unit_size: Option<f64>,
    #[pyo3(get)]
    geog_ellipsoid: Option<u16>,
    #[pyo3(get)]
    geog_semi_major_axis: Option<f64>,
    #[pyo3(get)]
    geog_semi_minor_axis: Option<f64>,
    #[pyo3(get)]
    geog_inv_flattening: Option<f64>,
    #[pyo3(get)]
    geog_azimuth_units: Option<u16>,
    #[pyo3(get)]
    geog_prime_meridian_long: Option<f64>,

    #[pyo3(get)]
    projected_type: Option<u16>,
    #[pyo3(get)]
    proj_citation: Option<String>,
    #[pyo3(get)]
    projection: Option<u16>,
    #[pyo3(get)]
    proj_coord_trans: Option<u16>,
    #[pyo3(get)]
    proj_linear_units: Option<u16>,
    #[pyo3(get)]
    proj_linear_unit_size: Option<f64>,
    #[pyo3(get)]
    proj_std_parallel1: Option<f64>,
    #[pyo3(get)]
    proj_std_parallel2: Option<f64>,
    #[pyo3(get)]
    proj_nat_origin_long: Option<f64>,
    #[pyo3(get)]
    proj_nat_origin_lat: Option<f64>,
    #[pyo3(get)]
    proj_false_easting: Option<f64>,
    #[pyo3(get)]
    proj_false_northing: Option<f64>,
    #[pyo3(get)]
    proj_false_origin_long: Option<f64>,
    #[pyo3(get)]
    proj_false_origin_lat: Option<f64>,
    #[pyo3(get)]
    proj_false_origin_easting: Option<f64>,
    #[pyo3(get)]
    proj_false_origin_northing: Option<f64>,
    #[pyo3(get)]
    proj_center_long: Option<f64>,
    #[pyo3(get)]
    proj_center_lat: Option<f64>,
    #[pyo3(get)]
    proj_center_easting: Option<f64>,
    #[pyo3(get)]
    proj_center_northing: Option<f64>,
    #[pyo3(get)]
    proj_scale_at_nat_origin: Option<f64>,
    #[pyo3(get)]
    proj_scale_at_center: Option<f64>,
    #[pyo3(get)]
    proj_azimuth_angle: Option<f64>,
    #[pyo3(get)]
    proj_straight_vert_pole_long: Option<f64>,

    #[pyo3(get)]
    vertical: Option<u16>,
    #[pyo3(get)]
    vertical_citation: Option<String>,
    #[pyo3(get)]
    vertical_datum: Option<u16>,
    #[pyo3(get)]
    vertical_units: Option<u16>,
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
