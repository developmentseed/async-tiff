from collections.abc import Buffer

from .enums import CompressionMethod
from ._decoder import DecoderRegistry
from ._thread_pool import ThreadPool

class Tile:
    """A representation of a TIFF image tile."""
    @property
    def x(self) -> int:
        """The column index this tile represents."""
    @property
    def y(self) -> int:
        """The row index this tile represents."""
    @property
    def compressed_bytes(self) -> Buffer:
        """The compressed bytes underlying this tile."""
    @property
    def compression_method(self) -> CompressionMethod | int:
        """The compression method used by this tile."""
    async def decode_async(
        self,
        *,
        decoder_registry: DecoderRegistry | None = None,
        pool: ThreadPool | None = None,
    ) -> Buffer:
        """Decode this tile's data.

        Keyword Args:
            decoder_registry: the decoders to use for decompression. Defaults to None.
            pool: the thread pool on which to run decompression. Defaults to None.

        Returns:
            Decoded tile data as a buffer.
        """
