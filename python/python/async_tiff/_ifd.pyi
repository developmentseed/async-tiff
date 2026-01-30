from collections.abc import Iterable, Sequence
from typing import Any

from ._colormap import Colormap
from ._geo import GeoKeyDirectory
from ._tile import Tile
from .enums import (
    Compression,
    PhotometricInterpretation,
    PlanarConfiguration,
    Predictor,
    ResolutionUnit,
    SampleFormat,
)

Value = int | float | str | tuple[int, int] | list[Value]

class ImageFileDirectory:
    def keys(self) -> list[str]:
        """A list of string keys representing the IFD fields."""
    def __eq__(self, value: object) -> bool: ...
    def __iter__(self) -> Iterable[str]:
        """An iterable of string keys representing the IFD fields."""
    def __getitem__(self, key: str) -> Any:
        """Access IFD fields by string key."""

    @property
    def new_subfile_type(self) -> int | None: ...
    @property
    def image_width(self) -> int:
        """The number of columns in the image, i.e., the number of pixels per row."""

    @property
    def image_height(self) -> int:
        """The number of rows of pixels in the image."""

    @property
    def bits_per_sample(self) -> list[int]: ...
    @property
    def compression(self) -> Compression | int:
        """Access the compression tag.

        An `int` will be returned if the compression is not one of the values in
        `Compression`.
        """
    @property
    def photometric_interpretation(self) -> PhotometricInterpretation: ...
    @property
    def document_name(self) -> str | None: ...
    @property
    def image_description(self) -> str | None: ...
    @property
    def strip_offsets(self) -> list[int] | None: ...
    @property
    def orientation(self) -> int | None: ...
    @property
    def samples_per_pixel(self) -> int:
        """
        The number of components per pixel.

        SamplesPerPixel is usually 1 for bilevel, grayscale, and palette-color images.
        SamplesPerPixel is usually 3 for RGB images. If this value is higher,
        ExtraSamples should give an indication of the meaning of the additional
        channels.
        """

    @property
    def rows_per_strip(self) -> int | None: ...
    @property
    def strip_byte_counts(self) -> int | None: ...
    @property
    def min_sample_value(self) -> int | None: ...
    @property
    def max_sample_value(self) -> int | None: ...
    @property
    def x_resolution(self) -> float | None:
        """The number of pixels per ResolutionUnit in the ImageWidth direction."""

    @property
    def y_resolution(self) -> float | None:
        """The number of pixels per ResolutionUnit in the ImageLength direction."""

    @property
    def planar_configuration(self) -> PlanarConfiguration: ...
    @property
    def resolution_unit(self) -> ResolutionUnit | None: ...
    @property
    def software(self) -> str | None: ...
    @property
    def date_time(self) -> str | None: ...
    @property
    def artist(self) -> str | None: ...
    @property
    def host_computer(self) -> str | None: ...
    @property
    def predictor(self) -> Predictor | None: ...
    @property
    def tile_width(self) -> int | None: ...
    @property
    def tile_height(self) -> int | None: ...
    @property
    def tile_offsets(self) -> list[int] | None: ...
    @property
    def tile_byte_counts(self) -> list[int] | None: ...
    @property
    def extra_samples(self) -> list[int] | None: ...
    @property
    def sample_format(self) -> list[SampleFormat]: ...
    @property
    def jpeg_tables(self) -> bytes | None: ...
    @property
    def copyright(self) -> str | None: ...
    @property
    def geo_key_directory(self) -> GeoKeyDirectory | None: ...
    @property
    def model_pixel_scale(self) -> list[float] | None: ...
    @property
    def model_tiepoint(self) -> list[float] | None: ...
    @property
    def model_transformation(self) -> list[float] | None: ...
    @property
    def gdal_nodata(self) -> str | None: ...
    @property
    def gdal_metadata(self) -> str | None: ...
    @property
    def other_tags(self) -> dict[int, Value]: ...
    @property
    def colormap(self) -> Colormap | None:
        """The colormap for palette-color images."""
        ...
    async def fetch_tile(self, x: int, y: int) -> Tile:
        """Fetch a single tile.

        Args:
            x: The column index within the ifd to read from.
            y: The row index within the ifd to read from.

        Returns:
            Tile response.
        """
    async def fetch_tiles(self, x: Sequence[int], y: Sequence[int]) -> list[Tile]:
        """Fetch multiple tiles concurrently.

        Args:
            x: The column indexes within the ifd to read from.
            y: The row indexes within the ifd to read from.

        Returns:
            Tile responses.
        """
