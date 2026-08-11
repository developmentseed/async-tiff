"""Tags whose values are IFD offsets, which the Python layer used to refuse.

`TagValue::Ifd` and `TagValue::IfdBig` were the only variants `PyValue` had no conversion for, so any
file carrying such a tag raised `RuntimeError: Unsupported value type 'Ifd'` — and it raised while the
directory was being read, before a caller could choose to ignore the tag. One tag made the whole file
unopenable.

The tag that matters in practice is 330, SubIFDs, which microscopy writers use for pyramid levels:
typed IFD in a classic TIFF and IFD8 in a BigTIFF. The Rust core already parsed both and converts
them to integers; only the binding declined.
"""

import numpy as np
import pytest
from async_tiff import TIFF
from async_tiff.store import LocalStore

tifffile = pytest.importorskip("tifffile")

SUBIFDS_TAG = 330


def write_with_subifd(path, *, bigtiff: bool) -> None:
    """A two-level TIFF whose reduced level is a SubIFD rather than a second top-level IFD."""
    full = np.arange(64 * 64, dtype=np.uint16).reshape(64, 64)
    with tifffile.TiffWriter(path, bigtiff=bigtiff) as writer:
        writer.write(full, subifds=1, tile=(16, 16))
        writer.write(full[::2, ::2], subfiletype=1, tile=(16, 16))


@pytest.mark.parametrize(
    ("bigtiff", "expected_type"),
    [(False, "IFD"), (True, "IFD8")],
    ids=["classic", "bigtiff"],
)
async def test_subifds_tag_reads_as_offsets(tmp_path, bigtiff, expected_type):
    """The tag arrives as integers, and the file opens.

    `expected_type` is the TIFF value type the writer uses, which is what distinguishes the two
    variants this covers; both used to raise.
    """
    path = tmp_path / f"subifd_{expected_type.lower()}.tif"
    write_with_subifd(path, bigtiff=bigtiff)
    with tifffile.TiffFile(path) as reference:
        assert reference.pages[0].tags[SUBIFDS_TAG].dtype_name == expected_type
        assert reference.is_bigtiff is bigtiff

    tiff = await TIFF.open(path.name, store=LocalStore(tmp_path))
    offsets = tiff.ifds[0].other_tags[SUBIFDS_TAG]

    if isinstance(offsets, int):
        offsets = [offsets]
    assert offsets, "the tag carried no offsets"
    assert all(isinstance(offset, int) for offset in offsets)
    # An offset points inside the file, past the header.
    assert all(0 < offset < path.stat().st_size for offset in offsets)


async def test_subifds_can_be_read_at_their_offsets(tmp_path):
    """And the levels they point at are reachable, which is the reason to expose the tag.

    `ifds` follows the top-level chain, which SubIFDs are not part of, so a pyramid written that way
    is invisible to a reader that only walks the chain -- one resolution out of however many.
    """
    path = tmp_path / "subifd_levels.tif"
    write_with_subifd(path, bigtiff=False)

    tiff = await TIFF.open(path.name, store=LocalStore(tmp_path))
    assert len(tiff.ifds) == 1, "the reduced level is not in the top-level chain"

    offsets = tiff.ifds[0].other_tags[SUBIFDS_TAG]
    if isinstance(offsets, int):
        offsets = [offsets]
    reduced = await tiff.ifd_at(offsets[0])
    assert (reduced.image_height, reduced.image_width) == (32, 32)


async def test_the_image_beside_the_tag_still_reads(tmp_path):
    """And the IFD carrying the tag is otherwise unaffected: its own tiles are where it says."""
    path = tmp_path / "subifd_pixels.tif"
    write_with_subifd(path, bigtiff=False)

    tiff = await TIFF.open(path.name, store=LocalStore(tmp_path))
    ifd = tiff.ifds[0]
    assert (ifd.image_height, ifd.image_width) == (64, 64)
    assert ifd.tile_count == (4, 4)
    assert len(ifd.tile_offsets) == 16
