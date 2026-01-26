import numpy as np
import pytest
from async_tiff import TIFF, enums
from async_tiff.store import LocalStore, S3Store


async def test_cog_s3():
    """
    Ensure that TIFF.open_async can open a Sentinel-2 Cloud-Optimized GeoTIFF file from
    an s3 bucket, read IFDs and GeoKeyDirectory metadata.
    """
    path = "sentinel-s2-l2a-cogs/12/S/UF/2022/6/S2B_12SUF_20220609_0_L2A/B04.tif"
    store = S3Store("sentinel-cogs", region="us-west-2", skip_signature=True)
    tiff = await TIFF.open_async(path=path, store=store)

    assert tiff.endianness == enums.Endianness.LittleEndian

    ifds = tiff.ifds
    assert len(ifds) == 5

    ifd = ifds[0]
    assert ifd.compression == enums.CompressionMethod.Deflate
    assert ifd.tile_height == 1024
    assert ifd.tile_width == 1024
    assert ifd.photometric_interpretation == enums.PhotometricInterpretation.BlackIsZero
    assert ifd.gdal_nodata == "0"
    assert (
        ifd.gdal_metadata
        == '<GDALMetadata>\n  <Item name="OVR_RESAMPLING_ALG">AVERAGE</Item>\n</GDALMetadata>\n'
    )

    gkd = ifd.geo_key_directory
    assert gkd is not None, "GeoKeyDirectory should exist"
    assert gkd.citation == "WGS 84 / UTM zone 12N"
    assert gkd.projected_type == 32612

    tile = await tiff.fetch_tile(0, 0, 0)
    array = await tile.decode()
    np_array = np.asarray(array, copy=False)
    assert np_array.shape == (1024, 1024, 1)


async def test_cog_missing_file():
    """
    Ensure that a FileNotFoundError is raised when passing in a missing file.
    """
    store = LocalStore()
    with pytest.raises(FileNotFoundError):
        await TIFF.open_async(path="imaginary_file.tif", store=store)
