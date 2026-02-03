from typing import Protocol

from obspec import GetRangeAsync, GetRangesAsync


class ObspecInput(GetRangeAsync, GetRangesAsync, Protocol):
    """Supported obspec input to reader.

    Anything that implements [GetRangeAsync][obspec.GetRangeAsync] and
    [GetRangesAsync][obspec.GetRangesAsync] can be used as an input to the TIFF reader.
    """
