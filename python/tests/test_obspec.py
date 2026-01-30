from collections.abc import Buffer, Sequence
from pathlib import Path

import pytest
from async_tiff import TIFF
from obstore.store import LocalStore

FIXTURES_DIR = Path(__file__).parent.parent.parent / "fixtures"


async def load_tiff(filename: str):
    store = LocalStore(FIXTURES_DIR)
    tiff = await TIFF.open(path=filename, store=store)
    return tiff


class ObstoreWrapper:
    store: LocalStore

    def __init__(self, store: LocalStore):
        self.store = store

    async def get_range_async(
        self,
        path: str,
        *,
        start: int,
        end: int | None = None,
        length: int | None = None,
    ) -> Buffer:
        """Call `get_range` asynchronously.

        Refer to the documentation for [get_range][obstore.get_range].
        """
        return await self.store.get_range_async(
            path,
            start=start,
            end=end,
            length=length,
        )

    async def get_ranges_async(
        self,
        path: str,
        *,
        starts: Sequence[int],
        ends: Sequence[int] | None = None,
        lengths: Sequence[int] | None = None,
    ) -> Sequence[Buffer]:
        """Call `get_ranges` asynchronously.

        Refer to the documentation for [get_ranges][obstore.get_ranges].
        """
        return await self.store.get_ranges_async(
            path,
            starts=starts,
            ends=ends,
            lengths=lengths,
        )


@pytest.mark.asyncio
async def test_read_with_obspec():
    store = LocalStore(FIXTURES_DIR)
    wrapper = ObstoreWrapper(store)

    filename = "other/geogtowgs_subset_USGS_13_s14w171.tif"
    tiff = await TIFF.open(path=filename, store=wrapper)
    assert len(tiff.ifds) > 0
