import tifffile

dir(tifffile)
tifffile.TiffFile
dir(tifffile.TiffFile)

path = "/Users/kyle/github/developmentseed/async-tiff/python/50N_120W.tif"
tif = tifffile.TiffFile(path)
tags = tif.pages[0].tags
strip_offsets = tags.get(273)
dir(strip_offsets)
strip_offsets.count
strip_offsets.dtype
dir(tags)
tags.items()


from async_tiff import TIFF
from async_tiff.store import HTTPStore
from urllib.parse import urlparse
import asyncio

year = 2000
file_name = "50N_120W"
url = f"https://storage.googleapis.com/earthenginepartners-hansen/GLCLU2000-2020/v2/{year}/{file_name}.tif"
parsed = urlparse(url)
store = HTTPStore.from_url(f"{parsed.scheme}://{parsed.netloc}")

tiff = await TIFF.open(path, store=store, prefetch=1024 * 1024)


async def open_tiff(*, store, path):
    return


tiff = asyncio.run(open_tiff(store=store, path=parsed.path))
print(tiff)
