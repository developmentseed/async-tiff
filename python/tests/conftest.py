from __future__ import annotations

from contextlib import contextmanager
from pathlib import Path
from typing import TYPE_CHECKING, Any, Generator, Protocol

import pytest
import rasterio
from async_tiff import TIFF
from async_tiff.store import LocalStore

if TYPE_CHECKING:
    from rasterio.io import DatasetReader


class LoadTIFF(Protocol):
    async def __call__(
        self,
        name: str,
        *,
        variant: str,
    ) -> TIFF: ...


class LoadRasterio(Protocol):
    @contextmanager
    def __call__(
        self,
        name: str,
        *,
        variant: str,
        OVERVIEW_LEVEL: int | None = None,  # noqa: N803
        **kwargs: Any,  # noqa: ANN401
    ) -> Generator[DatasetReader, None, None]: ...


@pytest.fixture(scope="session")
def root_dir() -> Path:
    root_dir = Path(__file__).parent.resolve()

    while root_dir.name != "async-tiff":
        root_dir = root_dir.parent

    return root_dir


@pytest.fixture(scope="session")
def fixture_store(root_dir) -> LocalStore:
    return LocalStore(root_dir / "fixtures")


@pytest.fixture
def load_tiff(fixture_store):
    async def _load(name: str, *, variant: str) -> TIFF:
        path = "geotiff-test-data/"

        if variant == "rasterio":
            path += "rasterio_generated/fixtures/"
        else:
            path += f"real_data/{variant}/"

        path = f"{path}{name}.tif"
        return await TIFF.open(path=path, store=fixture_store)

    return _load


@pytest.fixture
def load_rasterio(root_dir):
    @contextmanager
    def _load(
        name: str,
        *,
        variant: str,
        OVERVIEW_LEVEL: int | None = None,  # noqa: N803
        **kwargs: Any,  # noqa: ANN401
    ) -> Generator[DatasetReader, None, None]:
        path = f"{root_dir}/fixtures/geotiff-test-data/"
        if variant == "rasterio":
            path += "rasterio_generated/fixtures/"
        else:
            path += f"real_data/{variant}/"

        path = f"{path}{name}.tif"

        if OVERVIEW_LEVEL is not None and "OVERVIEW_LEVEL" not in kwargs:
            kwargs["OVERVIEW_LEVEL"] = OVERVIEW_LEVEL

        with rasterio.open(path, **kwargs) as ds:
            yield ds

    return _load
