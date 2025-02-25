from enum import IntEnum


class CompressionMethod(IntEnum):
    """
    See [TIFF compression
    tags](https://www.awaresystems.be/imaging/tiff/tifftags/compression.html) for
    reference.
    """

    Uncompressed = 1
    Huffman = 2
    Fax3 = 3
    Fax4 = 4
    LZW = 5
    JPEG = 6
    # // "Extended JPEG" or "new JPEG" style
    ModernJPEG = 7
    Deflate = 8
    OldDeflate = 0x80B2
    PackBits = 0x8005


class PhotometricInterpretation(IntEnum):
    WhiteIsZero = 0
    BlackIsZero = 1
    RGB = 2
    RGBPalette = 3
    TransparencyMask = 4
    CMYK = 5
    YCbCr = 6
    CIELab = 8


class PlanarConfiguration(IntEnum):
    Chunky = 1
    Planar = 2


class Predictor(IntEnum):
    Unknown = 1
    Horizontal = 2
    FloatingPoint = 3


class ResolutionUnit(IntEnum):
    Unknown = 1
    Inch = 2
    Centimeter = 3


class SampleFormat(IntEnum):
    Uint = 1
    Int = 2
    IEEEFP = 3
    Void = 4
