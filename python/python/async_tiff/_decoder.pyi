from typing import Protocol
from collections.abc import Buffer

from .enums import CompressionMethod

class Decoder(Protocol):
    @staticmethod
    def __call__(buffer: Buffer) -> Buffer: ...

class DecoderRegistry:
    def __init__(self) -> None: ...
    def add(self, compression: CompressionMethod | int, decoder: Decoder) -> None: ...
