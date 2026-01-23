import numpy as np
import pytest
from async_tiff import Array


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
