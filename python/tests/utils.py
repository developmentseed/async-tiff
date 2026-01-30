from pathlib import Path

from async_tiff import TIFF
from async_tiff.store import LocalStore

FIXTURES_DIR = Path(__file__).parent.parent.parent / "fixtures"


async def load_tiff(filename: str):
    store = LocalStore(FIXTURES_DIR)
    tiff = await TIFF.open(path=filename, store=store)
    return tiff
