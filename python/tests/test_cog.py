import async_tiff
from time import time

import pytest

from async_tiff import TIFF
from async_tiff.store import LocalStore, S3Store

store = S3Store("sentinel-cogs", region="us-west-2", skip_signature=True)
path = "sentinel-s2-l2a-cogs/12/S/UF/2022/6/S2B_12SUF_20220609_0_L2A/B04.tif"

tiff = await TIFF.open(path, store=store, prefetch=32768)

start = time()
tiff = await TIFF.open(path, store=store, prefetch=32768)
end = time()
end - start

ifds = tiff.ifds
ifd = ifds[0]
ifd.compression
ifd.tile_height
ifd.tile_width
ifd.photometric_interpretation
gkd = ifd.geo_key_directory
gkd.citation
gkd.projected_type
gkd.citation

dir(gkd)


async def test_cog_missing_file():
    """
    Ensure that a FileNotFoundError is raised when passing in a missing file.
    """
    store = LocalStore()
    with pytest.raises(FileNotFoundError):
        await TIFF.open(path="imaginary_file.tif", store=store)
