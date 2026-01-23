from typing import Protocol
from collections.abc import Buffer


class Decoder(Protocol):
    """A custom Python-provided decompression algorithm."""

    # In the future, we could pass in photometric interpretation and jpeg tables as
    # well.
    @staticmethod
    def __call__(buffer: Buffer) -> Buffer:
        """A callback to decode compressed data."""
        ...
