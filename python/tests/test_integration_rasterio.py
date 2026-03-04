from __future__ import annotations

from typing import TYPE_CHECKING

import numpy as np
import pytest
from async_tiff.enums import PlanarConfiguration
from rasterio.plot import reshape_as_image
from rasterio.windows import Window

if TYPE_CHECKING:
    from .conftest import LoadRasterio, LoadTIFF


@pytest.mark.asyncio
@pytest.mark.parametrize(
    ("variant", "file_name"),
    [
        ("eox", "eox_cloudless"),
        ("rasterio", "uint8_1band_deflate_block128_unaligned_predictor2"),
    ],
)
async def test_read(
    load_tiff: LoadTIFF,
    load_rasterio: LoadRasterio,
    variant: str,
    file_name: str,
) -> None:
    tiff = await load_tiff(file_name, variant=variant)
    ifd = tiff.ifds[0]

    tile_count = ifd.tile_count
    tile_width = ifd.tile_width
    tile_height = ifd.tile_height

    assert tile_count is not None
    assert tile_width is not None
    assert tile_height is not None

    x_count, y_count = tile_count

    with load_rasterio(file_name, variant=variant) as rasterio_ds:
        for x in range(x_count):
            for y in range(y_count):
                tile = await ifd.fetch_tile(x, y)
                array = await tile.decode()
                data = np.array(array)

                rasterio_window = create_window(tile_width, tile_height, x, y)

                rasterio_data = rasterio_ds.read(window=rasterio_window, boundless=True)

                if ifd.planar_configuration == PlanarConfiguration.Chunky:
                    np.testing.assert_array_equal(data, reshape_as_image(rasterio_data))
                else:
                    np.testing.assert_array_equal(data, rasterio_data)


def create_window(tile_width: int, tile_height: int, x: int, y: int) -> Window:
    return Window(
        x * tile_width,
        y * tile_height,
        tile_width,
        tile_height,
    )
