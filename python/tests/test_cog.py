from async_tiff import TIFF, enums
from async_tiff.store import S3Store


async def test_cog_s3():
    """
    Ensure that TIFF.open can open a Sentinel-2 Cloud-Optimized GeoTIFF file from an
    s3 bucket, read IFDs and GeoKeyDirectory metadata.
    """
    path = "sentinel-s2-l2a-cogs/12/S/UF/2022/6/S2B_12SUF_20220609_0_L2A/B04.tif"
    store = S3Store("sentinel-cogs", region="us-west-2", skip_signature=True)
    tiff = await TIFF.open(path=path, store=store, prefetch=32768)

    ifds = tiff.ifds
    assert len(ifds) == 5

    ifd = ifds[0]
    assert ifd.compression == enums.CompressionMethod.Deflate
    assert ifd.tile_height == 1024
    assert ifd.tile_width == 1024
    assert ifd.photometric_interpretation == enums.PhotometricInterpretation.BlackIsZero

    gkd = ifd.geo_key_directory
    assert gkd.citation == "WGS 84 / UTM zone 12N"
    assert gkd.projected_type == 32612
