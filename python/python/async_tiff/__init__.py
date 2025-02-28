from typing import TYPE_CHECKING

from ._async_tiff import *
from ._async_tiff import ___version

if TYPE_CHECKING:
    from . import store

__version__: str = ___version()
