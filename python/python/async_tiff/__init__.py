from typing import TYPE_CHECKING

from ._async_tiff import (
    TIFF,
    Array,
    DecoderRegistry,
    GeoKeyDirectory,
    ImageFileDirectory,
    ThreadPool,
    Tile,
    ___version,  # noqa: F403 # pyright:ignore[reportAttributeAccessIssue]
)
from ._decoder_runtime import Decoder
from ._input import ObspecInput

if TYPE_CHECKING:
    from . import store

__version__: str = ___version()

__all__ = [
    "store",
    "Array",
    "Decoder",
    "DecoderRegistry",
    "GeoKeyDirectory",
    "ImageFileDirectory",
    "ThreadPool",
    "TIFF",
    "ObspecInput",
    "Tile",
]
