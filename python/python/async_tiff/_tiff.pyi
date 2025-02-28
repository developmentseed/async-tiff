from ._ifd import ImageFileDirectory
from .store import ObjectStore

class TIFF:
    @classmethod
    async def open(
        cls, path: str, *, store: ObjectStore, prefetch: int | None = 16384
    ) -> TIFF: ...
    @property
    def ifds(self) -> list[ImageFileDirectory]: ...
