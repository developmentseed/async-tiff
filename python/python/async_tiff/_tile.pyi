from collections.abc import Buffer

from ._decoder import DecoderRegistry
from ._thread_pool import ThreadPool

class Tile:
    async def decode(
        self,
        *,
        decoder_registry: DecoderRegistry | None = None,
        pool: ThreadPool | None = None,
    ) -> Buffer: ...
