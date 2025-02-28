from typing import Protocol
from collections.abc import Buffer

from .enums import CompressionMethod

class Decoder(Protocol):
    # In the future, we could pass in photometric interpretation and jpeg tables as
    # well.
    @staticmethod
    def __call__(buffer: Buffer) -> Buffer: ...

class DecoderRegistry:
    def __init__(
        self, decoders: dict[CompressionMethod | int, Decoder] | None = None
    ) -> None: ...
