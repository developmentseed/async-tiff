import sys

if sys.version_info >= (3, 12):
    from collections.abc import Buffer
else:
    from typing_extensions import Buffer

class Colormap(Buffer):
    """A 1D array of u16 values representing a TIFF colormap.

    Implements Python's buffer protocol for zero-copy access via `np.asarray()`.
    """
    def __buffer__(self, flags: int) -> memoryview[int]: ...
    def __len__(self) -> int: ...
