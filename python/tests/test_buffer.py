from async_tiff._async_tiff import example_array
import numpy as np

rust_array = example_array()
np_view = np.array(rust_array)
np_view
# array([[1, 2, 3],
#        [4, 5, 6]], dtype=uint8)
