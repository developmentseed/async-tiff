import numpy as np
import pytest
from async_tiff import Array
from async_tiff.enums import PhotometricInterpretation

from .utils import load_tiff


@pytest.mark.parametrize(
    "dtype,format_str",
    [
        (np.uint8, "<B"),
        (np.uint16, "<H"),
        (np.uint32, "<I"),
        (np.uint64, "<Q"),
        (np.int8, "<b"),
        (np.int16, "<h"),
        (np.int32, "<i"),
        (np.int64, "<q"),
        (np.float32, "<f"),
        (np.float64, "<d"),
    ],
)
def test_round_trip(dtype, format_str):
    """Test round-trip conversion for all supported dtypes."""
    np_array = np.array([[[1, 2, 3], [4, 5, 6]]], dtype=dtype)
    assert np_array.shape == (1, 2, 3)

    shape = np_array.shape[0], np_array.shape[1], np_array.shape[2]
    rust_array = Array(np_array.tobytes(), shape=shape, format=format_str)

    np_view = np.asarray(rust_array)
    assert np_view.shape == np_array.shape
    assert np_view.dtype == np_array.dtype
    assert np.array_equal(np_array, np_view)


async def test_loading_bitmask():
    tiff = await load_tiff(
        "geotiff-test-data/real_data/vantor/maxar_opendata_yellowstone_visual.tif"
    )

    ifd_index = 1
    ifd = tiff.ifds[ifd_index]

    assert ifd.bits_per_sample == [1]
    assert ifd.photometric_interpretation == PhotometricInterpretation.TransparencyMask

    # IFD 1 is a bitmask
    tile = await tiff.fetch_tile(0, 0, ifd_index)
    array = await tile.decode()
    assert array.shape == (64, 64, 1)

    arr = np.asarray(array)
    assert list(np.unique(arr)) == [1]
