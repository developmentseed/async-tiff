import numpy as np

from .utils import load_tiff


async def test_ifd_dict():
    name = "geotiff-test-data/real_data/nlcd/nlcd_landcover.tif"
    tiff = await load_tiff(name)
    first_ifd = tiff.ifds[0]
    cmap = np.asarray(first_ifd.colormap, copy=False)
    assert cmap.shape == (256, 3)
