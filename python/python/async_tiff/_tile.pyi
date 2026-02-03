from collections.abc import Buffer

from ._array import Array
from ._decoder import DecoderRegistry
from ._thread_pool import ThreadPool
from .enums import Compression

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
    def compression_method(self) -> Compression | int:
        """The compression method used by this tile."""
    def decode_sync(
        self,
        *,
        decoder_registry: DecoderRegistry | None = None,
    ) -> Array:
        """Decode this tile's data.

        **Note**: This is a blocking function and will perform the tile decompression on
        the current thread. Prefer using the asynchronous `decode` method, which will
        offload decompression to a thread pool.

        Keyword Args:
            decoder_registry: the decoders to use for decompression. Defaults to None, in which case a default decoder registry is used.

        Returns:
            Decoded tile data as an Array instance.
        """

    async def decode(
        self,
        *,
        decoder_registry: DecoderRegistry | None = None,
        pool: ThreadPool | None = None,
    ) -> Array:
        """Decode this tile's data.

        This is an asynchronous function that will offload the tile decompression to a
        thread pool.

        Keyword Args:
            decoder_registry: the decoders to use for decompression. Defaults to None, in which case a default decoder registry is used.
            pool: the thread pool on which to run decompression. Defaults to None, in
                which case, a default thread pool is used.

        Returns:
            Decoded tile data as an Array instance.
        """
