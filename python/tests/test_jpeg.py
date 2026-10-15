"""Compare JPEG tiles decoded by async-tiff with rasterio.

JPEG decoders are not bit-exact with each other, so these comparisons allow a small tolerance:

- Without chroma subsampling, the IDCT rounds differently by at most 1.
- With 2x2 YCbCr subsampling, zune-jpeg rounds its chroma upsampling twice where libjpeg-turbo
  rounds once, which after YCbCr to RGB conversion differs by up to 5.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import numpy as np
import pytest
import rasterio
from async_tiff import TIFF
from async_tiff.enums import PlanarConfiguration
from async_tiff.store import LocalStore
from packaging.version import Version
from rasterio.windows import Window

if TYPE_CHECKING:
    from pathlib import Path

    from async_tiff import ImageFileDirectory
    from rasterio.io import DatasetReader

    from .conftest import LoadRasterio, LoadTIFF

# rasterio 1.5 wheels decode JPEG with libjpeg-turbo, but 1.4 wheels (the newest for Python 3.11)
# bundle IJG libjpeg 9. It upsamples YCbCr chroma by DCT scaling instead of libjpeg-turbo's
# triangle filter, so its YCbCr output differs from libjpeg-turbo, zune-jpeg and jpeg-decoder alike
# by up to ~90, mostly at block edges. A tolerance that loose would hide real bugs.
requires_libjpeg_turbo = pytest.mark.skipif(
    Version(rasterio.__version__) < Version("1.5"),
    reason="rasterio < 1.5 wheels decode YCbCr JPEG with IJG libjpeg 9",
)


async def assert_tiles_match(
    ifd: ImageFileDirectory,
    rasterio_ds: DatasetReader,
    *,
    atol: int,
) -> None:
    """Compare every tile of `ifd` with the same window read by rasterio.

    Edge tiles are only compared inside the image: JPEG always encodes whole tiles, and the
    padding past the image edge is not image data.
    """
    tile_width = ifd.tile_width
    tile_height = ifd.tile_height
    tile_count = ifd.tile_count
    assert tile_width is not None
    assert tile_height is not None
    assert tile_count is not None

    x_count, y_count = tile_count
    for x in range(x_count):
        for y in range(y_count):
            tile = await ifd.fetch_tile(x, y)
            data = np.array(await tile.decode())

            cols = min(tile_width, ifd.image_width - x * tile_width)
            rows = min(tile_height, ifd.image_height - y * tile_height)
            window = Window(x * tile_width, y * tile_height, cols, rows)
            expected = rasterio_ds.read(window=window)

            if ifd.planar_configuration == PlanarConfiguration.Chunky:
                data = data[:rows, :cols]
                expected = np.moveaxis(expected, 0, -1)
            else:
                data = data[:, :rows, :cols]

            np.testing.assert_allclose(
                data.astype(np.int16),
                expected.astype(np.int16),
                rtol=0,
                atol=atol,
            )


@pytest.mark.asyncio
@pytest.mark.filterwarnings("ignore::rasterio.errors.NotGeoreferencedWarning")
@pytest.mark.parametrize(
    ("photometric", "count", "interleave", "atol"),
    [
        ("RGB", 3, "pixel", 1),
        ("RGB", 3, "band", 1),
        ("MINISBLACK", 1, "pixel", 1),
        ("CMYK", 4, "pixel", 1),
        pytest.param("YCBCR", 3, "pixel", 5, marks=requires_libjpeg_turbo),
    ],
)
async def test_jpeg_matches_rasterio(
    tmp_path: Path,
    photometric: str,
    count: int,
    interleave: str,
    atol: int,
) -> None:
    # 200x150 with 64px tiles, so the last tile column and row are partial.
    height, width = 150, 200
    rng = np.random.default_rng(0)
    yy, xx = np.mgrid[0:height, 0:width]
    bands = [
        (xx * (i + 1) + yy * (3 - i)) % 256 + rng.normal(0, 12, (height, width))
        for i in range(count)
    ]
    data = np.clip(bands, 0, 255).astype(np.uint8)

    path = tmp_path / "jpeg.tif"
    with rasterio.open(
        path,
        "w",
        driver="GTiff",
        width=width,
        height=height,
        count=count,
        dtype="uint8",
        tiled=True,
        blockxsize=64,
        blockysize=64,
        compress="JPEG",
        photometric=photometric,
        interleave=interleave,
    ) as ds:
        ds.write(data)

    tiff = await TIFF.open(path=path.name, store=LocalStore(tmp_path))

    # GDAL converts JPEG-compressed CMYK to RGBA unless the file is opened in raw mode.
    rasterio_path = f"GTIFF_RAW:{path}" if photometric == "CMYK" else path
    with rasterio.open(rasterio_path) as ds:
        await assert_tiles_match(tiff.ifds[0], ds, atol=atol)


@pytest.mark.asyncio
@requires_libjpeg_turbo
@pytest.mark.parametrize(
    ("variant", "file_name"),
    [
        ("rio-tiler", "cog_rgb_with_stats"),
        ("vantor", "maxar_opendata_yellowstone_visual"),
    ],
)
async def test_jpeg_ycbcr_fixtures_match_rasterio(
    load_tiff: LoadTIFF,
    load_rasterio: LoadRasterio,
    variant: str,
    file_name: str,
) -> None:
    tiff = await load_tiff(file_name, variant=variant)
    with load_rasterio(file_name, variant=variant) as ds:
        await assert_tiles_match(tiff.ifds[0], ds, atol=5)
