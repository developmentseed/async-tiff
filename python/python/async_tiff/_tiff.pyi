from typing import Protocol
from ._tile import Tile
from ._ifd import ImageFileDirectory
from .store import ObjectStore

# Fix exports
from obspec._get import GetRangeAsync, GetRangesAsync

class ObspecInput(GetRangeAsync, GetRangesAsync, Protocol):
    """Supported obspec input to reader."""

class TIFF:
    @classmethod
    async def open(
        cls,
        path: str,
        *,
        store: ObjectStore | ObspecInput,
        prefetch: int = 32768,
    ) -> TIFF:
        """Open a new TIFF.

        Args:
            path: The path within the store to read from.
            store: The backend to use for data fetching.
            prefetch: The number of initial bytes to read up front.

        Returns:
            A TIFF instance.
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
    async def fetch_tiles(self, x: list[int], y: list[int], z: int) -> list[Tile]:
        """Fetch multiple tiles concurrently.

        Args:
            x: The column indexes within the ifd to read from.
            y: The row indexes within the ifd to read from.
            z: The IFD index to read from.

        Returns:
            Tile responses.
        """
