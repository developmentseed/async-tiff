from typing import TYPE_CHECKING

from ._async_tiff import *  # noqa: F403
from ._async_tiff import ___version

if TYPE_CHECKING:
    from . import store  # noqa: F401

__version__: str = ___version()
