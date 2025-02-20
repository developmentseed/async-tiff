import numpy as np
from rasterio.plot import show, reshape_as_raster

with open("img.buf", "rb") as f:
    buffer = f.read()

arr = np.frombuffer(buffer, np.uint8)
arr = arr.reshape(512, 512, 3)
arr = reshape_as_raster(arr)
show(arr, adjust=True)

###########

with open("img_from_tiff.buf", "rb") as f:
    buffer = f.read()

arr = np.frombuffer(buffer, np.uint8)
arr.reshape()

# We first need to reshape into "image" axes, then we need to reshape as "raster" axes.
arr = arr.reshape(12410, 9680, 3)
arr2 = reshape_as_raster(arr)
show(arr2, adjust=True)
