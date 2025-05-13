"""
Unit tests to ensure that proper errors are raised instead of a panic.
"""

import pytest
from async_tiff.store import HTTPStore

from async_tiff import TIFF


async def test_raise_typeerror_fetch_tile_striped_tiff():
    """
    Ensure that a TypeError is raised when trying to fetch a tile from a striped TIFF.
    """
    store = HTTPStore(url="https://github.com/")
    path = "OSGeo/gdal/raw/refs/tags/v3.11.0/autotest/gdrivers/data/gtiff/int8.tif"

    tiff = await TIFF.open(path=path, store=store)
    assert len(tiff.ifds) >= 1

    with pytest.raises(TypeError):
        await tiff.fetch_tile(0, 0, 0)
