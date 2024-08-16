use byteorder::LittleEndian;
use tiff::decoder::ifd::{Directory, Value};
use tiff::encoder::Rational;
use tiff::tags::{CompressionMethod, PhotometricInterpretation, ResolutionUnit, Tag};

use crate::cursor::ObjectStoreCursor;

/// A collection of all the IFD
// TODO: maybe separate out the primary/first image IFD out of the vec, as that one should have
// geospatial metadata?
pub(crate) struct ImageFileDirectories {
    /// There's always at least one IFD in a TIFF. We store this separately
    image_ifds: Vec<ImageIFD>,

    // Is it guaranteed that if masks exist that there will be one per image IFD? Or could there be
    // different numbers of image ifds and mask ifds?
    mask_ifds: Option<Vec<MaskIFD>>,
}

impl ImageFileDirectories {
    pub(crate) async fn open(cursor: &mut ObjectStoreCursor, ifd_offset: usize) -> Self {
        let mut next_ifd_offset = Some(ifd_offset);

        let mut image_ifds = vec![];
        let mut mask_ifds = vec![];
        while let Some(offset) = next_ifd_offset {
            let ifd = ImageFileDirectory::read(cursor, offset).await;
            next_ifd_offset = ifd.next_ifd_offset();
            match ifd {
                ImageFileDirectory::Image(image_ifd) => image_ifds.push(image_ifd),
                ImageFileDirectory::Mask(mask_ifd) => mask_ifds.push(mask_ifd),
            }
        }

        Self {
            image_ifds,
            // TODO: if empty, return None
            mask_ifds: Some(mask_ifds),
        }
    }
}

fn value_as_usize(value: &Value) -> usize {
    match value {
        Value::Byte(v) => *v as usize,
        Value::Short(v) => *v as usize,
        Value::Signed(v) => *v as usize,
        Value::SignedBig(v) => *v as usize,
        Value::Unsigned(v) => *v as usize,
        Value::UnsignedBig(v) => *v as usize,
        _ => panic!("Not an integer"),
    }
}

struct OptionalTags {
    directory: Directory,
}

impl OptionalTags {
    /// Check if the IFD contains a full resolution image (not an overview)
    fn is_full_resolution(&self) -> bool {
        if let Some(val) = self.directory.get(&Tag::NewSubfileType) {
            // if self.NewSubfileType.value[0] == 0:
            todo!()
        } else {
            true
        }
        // self.directory.contains_key(T)
    }
}

/// An ImageFileDirectory representing Image content
// TODO: required tags should be stored as rust-native types, not Value
struct ImageIFD {
    // Required tags
    /// The number of columns in the image, i.e., the number of pixels per row.
    image_width: u32,

    /// The number of rows of pixels in the image.
    image_height: u32,

    bits_per_sample: Vec<u16>,

    compression: CompressionMethod,

    photometric_interpretation: PhotometricInterpretation,

    strip_offsets: Option<u32>,

    samples_per_pixel: u16,

    rows_per_strip: Option<u32>,

    strip_byte_counts: Option<u32>,

    x_resolution: Rational,

    y_resolution: Rational,

    resolution_unit: ResolutionUnit,

    planar_configuration: Value,
    sample_format: Value,
    tile_byte_counts: Value,
    tile_height: Value,
    tile_offsets: Value,
    tile_width: Value,

    directory: Directory,
    next_ifd_offset: Option<usize>,
}

impl ImageIFD {
    // fn image_height(&self) -> u32 {
    //     match self.image_height {
    //         Value::Short(val) => val as u32,
    //         Value::Unsigned(val) => val,
    //         _ => unreachable!(),
    //     }
    // }

    // fn image_width(&self) -> u32 {
    //     match self.image_width {
    //         Value::Short(val) => val as u32,
    //         Value::Unsigned(val) => val,
    //         _ => unreachable!(),
    //     }
    // }

    // fn bands(&self) -> usize {
    //     value_as_usize(&self.samples_per_pixel)
    // }
}

/// An ImageFileDirectory representing Mask content
struct MaskIFD {
    next_ifd_offset: Option<usize>,
}

enum ImageFileDirectory {
    Image(ImageIFD),
    Mask(MaskIFD),
}

impl ImageFileDirectory {
    async fn read(cursor: &mut ObjectStoreCursor, offset: usize) -> Self {
        let ifd_start = offset;
        cursor.seek(offset);

        let tag_count = cursor.read_u16::<LittleEndian>().await;

        // let mut tags = HashMap::with_capacity(tag_count);
        for i in 0..tag_count {
            // todo: read tag
        }

        cursor.seek(ifd_start + (12 * tag_count as usize) + 2);

        let next_ifd_offset = cursor.read_u32::<LittleEndian>().await;
        let next_ifd_offset = if next_ifd_offset == 0 {
            None
        } else {
            Some(next_ifd_offset)
        };

        if is_masked_ifd() {
            Self::Mask(MaskIFD { next_ifd_offset })
        } else {
            todo!()
            // Self::Image(ImageIFD { next_ifd_offset })
        }
    }

    fn next_ifd_offset(&self) -> Option<usize> {
        match self {
            Self::Image(ifd) => ifd.next_ifd_offset,
            Self::Mask(ifd) => ifd.next_ifd_offset,
        }
    }
}

fn is_masked_ifd() -> bool {
    todo!()
    // https://github.com/geospatial-jeff/aiocogeo/blob/5a1d32c3f22c883354804168a87abb0a2ea1c328/aiocogeo/ifd.py#L66
}
