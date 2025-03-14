import obstore
from ._tile import Tile
from ._ifd import ImageFileDirectory
from .store import ObjectStore

class TIFF:
    @classmethod
    async def open(
        cls,
        path: str,
        *,
        store: obstore.store.ObjectStore | ObjectStore,
        prefetch: int | None = 16384,
    ) -> TIFF: ...
    @property
    def ifds(self) -> list[ImageFileDirectory]: ...
    async def fetch_tile(self, x: int, y: int, z: int) -> Tile: ...
    async def fetch_tiles(self, x: list[int], y: list[int], z: int) -> list[Tile]: ...
