from typing import TYPE_CHECKING

from ._async_tiff import (
    Array,
    Decoder,
    DecoderRegistry,
    GeoKeyDirectory,
    ImageFileDirectory,
    ThreadPool,
    TIFF,
    ObspecInput,
    Tile,
)

from ._async_tiff import ___version  # noqa: F403 # pyright:ignore[reportAttributeAccessIssue]

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
