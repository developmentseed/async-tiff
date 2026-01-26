from __future__ import annotations

import sys
from typing import TYPE_CHECKING, Protocol

if TYPE_CHECKING:
    if sys.version_info >= (3, 12):
        from collections.abc import Buffer
    else:
        from typing_extensions import Buffer


class Decoder(Protocol):
    """A custom Python-provided decompression algorithm."""

    # In the future, we could pass in photometric interpretation and jpeg tables as
    # well.
    @staticmethod
    def __call__(buffer: Buffer) -> Buffer:
        """A callback to decode compressed data."""
        ...
