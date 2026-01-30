from typing import Sequence

from ._ifd import ImageFileDirectory
from ._input import ObspecInput
from ._tile import Tile
from .enums import Endianness
from .store import ObjectStore

class TIFF:
    @classmethod
    async def open(
        cls,
        path: str,
        *,
        store: ObjectStore | ObspecInput,
        prefetch: int = 32768,
        multiplier: int | float = 2.0,
    ) -> TIFF:
        """Open a new TIFF.

        Args:
            path: The path within the store to read from.
            store: The backend to use for data fetching.
            prefetch: The number of initial bytes to read up front.
            multiplier: The multiplier to use for readahead size growth. Must be
                greater than 1.0. For example, for a value of `2.0`, the first metadata
                read will be of size `prefetch`, and then the next read will be of size
                `prefetch * 2`.

        Returns:
            A TIFF instance.
        """

    @property
    def endianness(self) -> Endianness:
        """The endianness of this TIFF file."""

    def ifd(self, index: int) -> ImageFileDirectory:
        """Access a specific IFD by index.

        Args:
            index: The IFD index to access.

        Returns:
            The requested IFD.
        """

    @property
    def ifds(self) -> list[ImageFileDirectory]:
        """Access the underlying IFDs of this TIFF.

        Each ImageFileDirectory (IFD) represents one of the internal "sub images" of
        this file.
        """
    async def fetch_tile(self, x: int, y: int, z: int) -> Tile:
        """Fetch a single tile.

        Args:
            x: The column index within the ifd to read from.
            y: The row index within the ifd to read from.
            z: The IFD index to read from.

        Returns:
            Tile response.
        """
    async def fetch_tiles(
        self, x: Sequence[int], y: Sequence[int], z: int
    ) -> list[Tile]:
        """Fetch multiple tiles concurrently.

        Args:
            x: The column indexes within the ifd to read from.
            y: The row indexes within the ifd to read from.
            z: The IFD index to read from.

        Returns:
            Tile responses.
        """
