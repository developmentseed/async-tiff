from typing import Protocol
from collections.abc import Buffer

from .enums import CompressionMethod

class Decoder(Protocol):
    """A custom Python-provided decompression algorithm."""
    # In the future, we could pass in photometric interpretation and jpeg tables as
    # well.
    @staticmethod
    def __call__(buffer: Buffer) -> Buffer:
        """A callback to decode compressed data."""

class DecoderRegistry:
    """A registry holding multiple decoder methods."""
    def __init__(
        self, custom_decoders: dict[CompressionMethod | int, Decoder] | None = None
    ) -> None:
        """Construct a new decoder registry.

        By default, pure-Rust decoders will be used for any recognized and supported
        compression types. Only the supplied decoders will override Rust-native
        decoders.

        Args:
            custom_decoders: any custom decoder methods to use. This will be applied
                _after_ (and override) any default provided Rust decoders. Defaults to
                None.
        """
