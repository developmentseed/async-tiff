from async_tiff import TIFF
from async_tiff.enums import (
    CompressionMethod,
    PhotometricInterpretation,
    PlanarConfiguration,
    SampleFormat,
)
from async_tiff.store import LocalStore
from pathlib import Path


FIXTURES_DIR = Path(__file__).parent.parent.parent / "tests" / "images"


async def load_tiff(filename: str):
    path = FIXTURES_DIR / filename
    store = LocalStore()
    tiff = await TIFF.open(path=str(path), store=store)
    return tiff


async def test_ifd_dict():
    filename = "geogtowgs_subset_USGS_13_s14w171.tif"
    tiff = await load_tiff(filename)
    first_ifd = tiff.ifds[0]

    expected_ifd = {
        "image_width": 1,
        "image_height": 1,
        "bits_per_sample": [32],
        "compression": CompressionMethod.Deflate,
        "photometric_interpretation": PhotometricInterpretation.BlackIsZero,
        "samples_per_pixel": 1,
        "planar_configuration": PlanarConfiguration.Chunky,
        "sample_format": [SampleFormat.IEEEFP],
        "other_tags": {},
        "strip_offsets": [8],
        "rows_per_strip": 1,
        "strip_byte_counts": [15],
        "geo_key_directory": first_ifd.geo_key_directory,
    }
    assert dict(first_ifd) == expected_ifd

    gkd = first_ifd.geo_key_directory
    assert gkd is not None
    expected_gkd = {
        "model_type": 2,
        "raster_type": 1,
        "geographic_type": 4269,
        "geog_citation": "NAD83",
        "geog_angular_units": 9102,
        "geog_semi_major_axis": 6378137.0,
        "geog_inv_flattening": 298.257222101004,
    }
    assert dict(gkd) == expected_gkd
