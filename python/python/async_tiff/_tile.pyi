from collections.abc import Buffer

from .enums import CompressionMethod
from ._decoder import DecoderRegistry
from ._thread_pool import ThreadPool

class Tile:
    @property
    def x(self) -> int: ...
    @property
    def y(self) -> int: ...
    @property
    def compressed_bytes(self) -> Buffer: ...
    @property
    def compression_method(self) -> CompressionMethod: ...
    async def decode(
        self,
        *,
        decoder_registry: DecoderRegistry | None = None,
        pool: ThreadPool | None = None,
    ) -> Buffer: ...
