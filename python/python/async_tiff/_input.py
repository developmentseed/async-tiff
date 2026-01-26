from typing import Protocol

# Fix exports
from obspec._get import GetRangeAsync, GetRangesAsync


class ObspecInput(GetRangeAsync, GetRangesAsync, Protocol):
    """Supported obspec input to reader."""
