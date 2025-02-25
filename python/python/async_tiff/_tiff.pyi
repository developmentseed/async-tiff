from typing import Any

from ._ifd import ImageFileDirectory

class TIFF:
    @classmethod
    async def open(
        cls, path: str, *, store: Any, prefetch: int | None = 16384
    ) -> TIFF: ...
    @property
    def ifds(self) -> list[ImageFileDirectory]: ...
