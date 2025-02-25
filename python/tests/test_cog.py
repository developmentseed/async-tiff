import async_tiff
from time import time
from async_tiff import TIFF
from async_tiff.store import S3Store

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
