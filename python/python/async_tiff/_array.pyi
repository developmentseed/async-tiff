import sys

if sys.version_info >= (3, 12):
    from collections.abc import Buffer
else:
    from typing_extensions import Buffer

class Array(Buffer):
    """A 3D array that implements Python's buffer protocol.

    This allows zero-copy interoperability with numpy via `np.asarray(arr)`.
    The array is immutable and exposes a read-only buffer.

    Example:

    ```python
    import numpy as np
    from async_tiff import Array

    # Create from raw bytes
    data = bytes([1, 2, 3, 4, 5, 6])
    arr = Array(data, shape=(1, 2, 3), format="<B")  # 1x2x3 uint8 array

    # Convert to numpy (zero-copy view)
    np_arr = np.asarray(arr)
    assert np_arr.shape == (1, 2, 3)
    assert np_arr.dtype == np.uint8
    ```
    """

    # This is intended only for tests
    # def __init__(
    #     self, data: Buffer, shape: tuple[int, int, int], format: str
    # ) -> None: ...
    def __buffer__(self, flags: int) -> memoryview[int]: ...
    @property
    def shape(self) -> tuple[int, int, int]:
        """The shape of the array.

        The interpretation depends on the PlanarConfiguration:

        - PlanarConfiguration=1 (chunky): (height, width, bands)
        - PlanarConfiguration=2 (planar): (bands, height, width)
        """
        ...
