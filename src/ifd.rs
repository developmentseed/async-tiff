use std::collections::HashMap;
use std::io::Read;
use std::ops::Range;

use bytes::{buf::Buf, Bytes};
use num_enum::TryFromPrimitive;

use crate::error::{AsyncTiffError, AsyncTiffResult};
use crate::geo::{GeoKeyDirectory, GeoKeyTag};
use crate::reader::{AsyncCursor, AsyncFileReader, EndianAwareReader};
use crate::tiff::tags::{
    CompressionMethod, PhotometricInterpretation, PlanarConfiguration, Predictor, ResolutionUnit,
    SampleFormat, Tag, Type,
};
use crate::tiff::{TiffError, Value};
use crate::tile::Tile;

const DOCUMENT_NAME: u16 = 269;

/// A collection of all the IFD
// TODO: maybe separate out the primary/first image IFD out of the vec, as that one should have
// geospatial metadata?
#[derive(Debug, Clone)]
pub struct ImageFileDirectories {
    /// There's always at least one IFD in a TIFF. We store this separately
    ifds: Vec<ImageFileDirectory>,
    // Is it guaranteed that if masks exist that there will be one per image IFD? Or could there be
    // different numbers of image ifds and mask ifds?
    // mask_ifds: Option<Vec<IFD>>,
}

impl AsRef<[ImageFileDirectory]> for ImageFileDirectories {
    fn as_ref(&self) -> &[ImageFileDirectory] {
        &self.ifds
    }
}

impl ImageFileDirectories {
    pub(crate) async fn open(
        cursor: &mut AsyncCursor,
        ifd_offset: u64,
        bigtiff: bool,
    ) -> AsyncTiffResult<Self> {
        let mut next_ifd_offset = Some(ifd_offset);

        let mut ifds = vec![];
        while let Some(offset) = next_ifd_offset {
            let ifd = ImageFileDirectory::read(cursor, offset, bigtiff).await?;
            next_ifd_offset = ifd.next_ifd_offset();
            ifds.push(ifd);
        }

        Ok(Self { ifds })
    }
}

/// An ImageFileDirectory representing Image content
// The ordering of these tags matches the sorted order in TIFF spec Appendix A
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ImageFileDirectory {
    pub(crate) new_subfile_type: Option<u32>,

    /// The number of columns in the image, i.e., the number of pixels per row.
    pub(crate) image_width: u32,

    /// The number of rows of pixels in the image.
    pub(crate) image_height: u32,

    pub(crate) bits_per_sample: Vec<u16>,

    pub(crate) compression: CompressionMethod,

    pub(crate) photometric_interpretation: PhotometricInterpretation,

    pub(crate) document_name: Option<String>,

    pub(crate) image_description: Option<String>,

    pub(crate) strip_offsets: Option<Vec<u64>>,

    pub(crate) orientation: Option<u16>,

    /// The number of components per pixel.
    ///
    /// SamplesPerPixel is usually 1 for bilevel, grayscale, and palette-color images.
    /// SamplesPerPixel is usually 3 for RGB images. If this value is higher, ExtraSamples should
    /// give an indication of the meaning of the additional channels.
    pub(crate) samples_per_pixel: u16,

    pub(crate) rows_per_strip: Option<u32>,

    pub(crate) strip_byte_counts: Option<Vec<u64>>,

    pub(crate) min_sample_value: Option<Vec<u16>>,
    pub(crate) max_sample_value: Option<Vec<u16>>,

    /// The number of pixels per ResolutionUnit in the ImageWidth direction.
    pub(crate) x_resolution: Option<f64>,

    /// The number of pixels per ResolutionUnit in the ImageLength direction.
    pub(crate) y_resolution: Option<f64>,

    /// How the components of each pixel are stored.
    ///
    /// The specification defines these values:
    ///
    /// - Chunky format. The component values for each pixel are stored contiguously. For example,
    ///   for RGB data, the data is stored as RGBRGBRGB
    /// - Planar format. The components are stored in separate component planes. For example, RGB
    ///   data is stored with the Red components in one component plane, the Green in another, and
    ///   the Blue in another.
    ///
    /// The specification adds a warning that PlanarConfiguration=2 is not in widespread use and
    /// that Baseline TIFF readers are not required to support it.
    ///
    /// If SamplesPerPixel is 1, PlanarConfiguration is irrelevant, and need not be included.
    pub(crate) planar_configuration: PlanarConfiguration,

    pub(crate) resolution_unit: Option<ResolutionUnit>,

    /// Name and version number of the software package(s) used to create the image.
    pub(crate) software: Option<String>,

    /// Date and time of image creation.
    ///
    /// The format is: "YYYY:MM:DD HH:MM:SS", with hours like those on a 24-hour clock, and one
    /// space character between the date and the time. The length of the string, including the
    /// terminating NUL, is 20 bytes.
    pub(crate) date_time: Option<String>,
    pub(crate) artist: Option<String>,
    pub(crate) host_computer: Option<String>,

    pub(crate) predictor: Option<Predictor>,

    /// A color map for palette color images.
    ///
    /// This field defines a Red-Green-Blue color map (often called a lookup table) for
    /// palette-color images. In a palette-color image, a pixel value is used to index into an RGB
    /// lookup table. For example, a palette-color pixel having a value of 0 would be displayed
    /// according to the 0th Red, Green, Blue triplet.
    ///
    /// In a TIFF ColorMap, all the Red values come first, followed by the Green values, then the
    /// Blue values. The number of values for each color is 2**BitsPerSample. Therefore, the
    /// ColorMap field for an 8-bit palette-color image would have 3 * 256 values. The width of
    /// each value is 16 bits, as implied by the type of SHORT. 0 represents the minimum intensity,
    /// and 65535 represents the maximum intensity. Black is represented by 0,0,0, and white by
    /// 65535, 65535, 65535.
    ///
    /// ColorMap must be included in all palette-color images.
    ///
    /// In Specification Supplement 1, support was added for ColorMaps containing other then RGB
    /// values. This scheme includes the Indexed tag, with value 1, and a PhotometricInterpretation
    /// different from PaletteColor then next denotes the colorspace of the ColorMap entries.
    pub(crate) color_map: Option<Vec<u16>>,

    pub(crate) tile_width: Option<u32>,
    pub(crate) tile_height: Option<u32>,

    pub(crate) tile_offsets: Option<Vec<u64>>,
    pub(crate) tile_byte_counts: Option<Vec<u64>>,

    pub(crate) extra_samples: Option<Vec<u16>>,

    pub(crate) sample_format: Vec<SampleFormat>,

    pub(crate) jpeg_tables: Option<Bytes>,

    pub(crate) copyright: Option<String>,

    // Geospatial tags
    pub(crate) geo_key_directory: Option<GeoKeyDirectory>,
    pub(crate) model_pixel_scale: Option<Vec<f64>>,
    pub(crate) model_tiepoint: Option<Vec<f64>>,

    // GDAL tags
    // no_data
    // gdal_metadata
    pub(crate) other_tags: HashMap<Tag, Value>,

    pub(crate) next_ifd_offset: Option<u64>,
}

impl ImageFileDirectory {
    /// Read and parse the IFD starting at the given file offset
    async fn read(
        cursor: &mut AsyncCursor,
        ifd_start: u64,
        bigtiff: bool,
    ) -> AsyncTiffResult<Self> {
        cursor.seek(ifd_start);

        let tag_count = if bigtiff {
            cursor.read_u64().await?
        } else {
            cursor.read_u16().await?.into()
        };
        let mut tags = HashMap::with_capacity(tag_count as usize);
        for _ in 0..tag_count {
            let (tag_name, tag_value) = read_tag(cursor, bigtiff).await?;
            tags.insert(tag_name, tag_value);
        }

        // Tag   2 bytes
        // Type  2 bytes
        // Count:
        //  - bigtiff: 8 bytes
        //  - else: 4 bytes
        // Value:
        //  - bigtiff: 8 bytes either a pointer the value itself
        //  - else: 4 bytes either a pointer the value itself
        let ifd_entry_byte_size = if bigtiff { 20 } else { 12 };
        // The size of `tag_count` that we read above
        let tag_count_byte_size = if bigtiff { 8 } else { 2 };

        // Reset the cursor position before reading the next ifd offset
        cursor.seek(ifd_start + (ifd_entry_byte_size * tag_count) + tag_count_byte_size);

        let next_ifd_offset = if bigtiff {
            cursor.read_u64().await?
        } else {
            cursor.read_u32().await?.into()
        };

        // If the ifd_offset is 0, stop
        let next_ifd_offset = if next_ifd_offset == 0 {
            None
        } else {
            Some(next_ifd_offset)
        };

        Self::from_tags(tags, next_ifd_offset)
    }

    fn next_ifd_offset(&self) -> Option<u64> {
        self.next_ifd_offset
    }

    fn from_tags(
        mut tag_data: HashMap<Tag, Value>,
        next_ifd_offset: Option<u64>,
    ) -> AsyncTiffResult<Self> {
        let mut new_subfile_type = None;
        let mut image_width = None;
        let mut image_height = None;
        let mut bits_per_sample = None;
        let mut compression = None;
        let mut photometric_interpretation = None;
        let mut document_name = None;
        let mut image_description = None;
        let mut strip_offsets = None;
        let mut orientation = None;
        let mut samples_per_pixel = None;
        let mut rows_per_strip = None;
        let mut strip_byte_counts = None;
        let mut min_sample_value = None;
        let mut max_sample_value = None;
        let mut x_resolution = None;
        let mut y_resolution = None;
        let mut planar_configuration = None;
        let mut resolution_unit = None;
        let mut software = None;
        let mut date_time = None;
        let mut artist = None;
        let mut host_computer = None;
        let mut predictor = None;
        let mut color_map = None;
        let mut tile_width = None;
        let mut tile_height = None;
        let mut tile_offsets = None;
        let mut tile_byte_counts = None;
        let mut extra_samples = None;
        let mut sample_format = None;
        let mut jpeg_tables = None;
        let mut copyright = None;
        let mut geo_key_directory_data = None;
        let mut model_pixel_scale = None;
        let mut model_tiepoint = None;
        let mut geo_ascii_params: Option<String> = None;
        let mut geo_double_params: Option<Vec<f64>> = None;

        let mut other_tags = HashMap::new();

        tag_data.drain().try_for_each(|(tag, value)| {
            match tag {
                Tag::NewSubfileType => new_subfile_type = Some(value.into_u32()?),
                Tag::ImageWidth => image_width = Some(value.into_u32()?),
                Tag::ImageLength => image_height = Some(value.into_u32()?),
                Tag::BitsPerSample => bits_per_sample = Some(value.into_u16_vec()?),
                Tag::Compression => {
                    compression = Some(CompressionMethod::from_u16_exhaustive(value.into_u16()?))
                }
                Tag::PhotometricInterpretation => {
                    photometric_interpretation =
                        PhotometricInterpretation::from_u16(value.into_u16()?)
                }
                Tag::ImageDescription => image_description = Some(value.into_string()?),
                Tag::StripOffsets => strip_offsets = Some(value.into_u64_vec()?),
                Tag::Orientation => orientation = Some(value.into_u16()?),
                Tag::SamplesPerPixel => samples_per_pixel = Some(value.into_u16()?),
                Tag::RowsPerStrip => rows_per_strip = Some(value.into_u32()?),
                Tag::StripByteCounts => strip_byte_counts = Some(value.into_u64_vec()?),
                Tag::MinSampleValue => min_sample_value = Some(value.into_u16_vec()?),
                Tag::MaxSampleValue => max_sample_value = Some(value.into_u16_vec()?),
                Tag::XResolution => match value {
                    Value::Rational(n, d) => x_resolution = Some(n as f64 / d as f64),
                    _ => unreachable!("Expected rational type for XResolution."),
                },
                Tag::YResolution => match value {
                    Value::Rational(n, d) => y_resolution = Some(n as f64 / d as f64),
                    _ => unreachable!("Expected rational type for YResolution."),
                },
                Tag::PlanarConfiguration => {
                    planar_configuration = PlanarConfiguration::from_u16(value.into_u16()?)
                }
                Tag::ResolutionUnit => {
                    resolution_unit = ResolutionUnit::from_u16(value.into_u16()?)
                }
                Tag::Software => software = Some(value.into_string()?),
                Tag::DateTime => date_time = Some(value.into_string()?),
                Tag::Artist => artist = Some(value.into_string()?),
                Tag::HostComputer => host_computer = Some(value.into_string()?),
                Tag::Predictor => predictor = Predictor::from_u16(value.into_u16()?),
                Tag::ColorMap => color_map = Some(value.into_u16_vec()?),
                Tag::TileWidth => tile_width = Some(value.into_u32()?),
                Tag::TileLength => tile_height = Some(value.into_u32()?),
                Tag::TileOffsets => tile_offsets = Some(value.into_u64_vec()?),
                Tag::TileByteCounts => tile_byte_counts = Some(value.into_u64_vec()?),
                Tag::ExtraSamples => extra_samples = Some(value.into_u16_vec()?),
                Tag::SampleFormat => {
                    let values = value.into_u16_vec()?;
                    sample_format = Some(
                        values
                            .into_iter()
                            .map(SampleFormat::from_u16_exhaustive)
                            .collect(),
                    );
                }
                Tag::JPEGTables => jpeg_tables = Some(value.into_u8_vec()?.into()),
                Tag::Copyright => copyright = Some(value.into_string()?),

                // Geospatial tags
                // http://geotiff.maptools.org/spec/geotiff2.4.html
                Tag::GeoKeyDirectoryTag => geo_key_directory_data = Some(value.into_u16_vec()?),
                Tag::ModelPixelScaleTag => model_pixel_scale = Some(value.into_f64_vec()?),
                Tag::ModelTiepointTag => model_tiepoint = Some(value.into_f64_vec()?),
                Tag::GeoAsciiParamsTag => geo_ascii_params = Some(value.into_string()?),
                Tag::GeoDoubleParamsTag => geo_double_params = Some(value.into_f64_vec()?),
                // Tag::GdalNodata
                // Tags for which the tiff crate doesn't have a hard-coded enum variant
                Tag::Unknown(DOCUMENT_NAME) => document_name = Some(value.into_string()?),
                _ => {
                    other_tags.insert(tag, value);
                }
            };
            Ok::<_, TiffError>(())
        })?;

        let mut geo_key_directory = None;

        // We need to actually parse the GeoKeyDirectory after parsing all other tags because the
        // GeoKeyDirectory relies on `GeoAsciiParamsTag` having been parsed.
        if let Some(data) = geo_key_directory_data {
            let mut chunks = data.chunks(4);

            let header = chunks
                .next()
                .expect("If the geo key directory exists, a header should exist.");
            let key_directory_version = header[0];
            assert_eq!(key_directory_version, 1);

            let key_revision = header[1];
            assert_eq!(key_revision, 1);

            let _key_minor_revision = header[2];
            let number_of_keys = header[3];

            let mut tags = HashMap::with_capacity(number_of_keys as usize);
            for _ in 0..number_of_keys {
                let chunk = chunks
                    .next()
                    .expect("There should be a chunk for each key.");

                let key_id = chunk[0];
                let tag_name =
                    GeoKeyTag::try_from_primitive(key_id).expect("Unknown GeoKeyTag id: {key_id}");

                let tag_location = chunk[1];
                let count = chunk[2];
                let value_offset = chunk[3];

                if tag_location == 0 {
                    tags.insert(tag_name, Value::Short(value_offset));
                } else if Tag::from_u16_exhaustive(tag_location) == Tag::GeoAsciiParamsTag {
                    // If the tag_location points to the value of Tag::GeoAsciiParamsTag, then we
                    // need to extract a subslice from GeoAsciiParamsTag

                    let geo_ascii_params = geo_ascii_params
                        .as_ref()
                        .expect("GeoAsciiParamsTag exists but geo_ascii_params does not.");
                    let value_offset = value_offset as usize;
                    let mut s = &geo_ascii_params[value_offset..value_offset + count as usize];

                    // It seems that this string subslice might always include the final |
                    // character?
                    if s.ends_with('|') {
                        s = &s[0..s.len() - 1];
                    }

                    tags.insert(tag_name, Value::Ascii(s.to_string()));
                } else if Tag::from_u16_exhaustive(tag_location) == Tag::GeoDoubleParamsTag {
                    // If the tag_location points to the value of Tag::GeoDoubleParamsTag, then we
                    // need to extract a subslice from GeoDoubleParamsTag

                    let geo_double_params = geo_double_params
                        .as_ref()
                        .expect("GeoDoubleParamsTag exists but geo_double_params does not.");
                    let value_offset = value_offset as usize;
                    let value = if count == 1 {
                        Value::Double(geo_double_params[value_offset])
                    } else {
                        let x = geo_double_params[value_offset..value_offset + count as usize]
                            .iter()
                            .map(|val| Value::Double(*val))
                            .collect();
                        Value::List(x)
                    };
                    tags.insert(tag_name, value);
                }
            }
            geo_key_directory = Some(GeoKeyDirectory::from_tags(tags)?);
        }

        let samples_per_pixel = samples_per_pixel.expect("samples_per_pixel not found");
        let planar_configuration = if let Some(planar_configuration) = planar_configuration {
            planar_configuration
        } else if samples_per_pixel == 1 {
            // If SamplesPerPixel is 1, PlanarConfiguration is irrelevant, and need not be included.
            // https://web.archive.org/web/20240329145253/https://www.awaresystems.be/imaging/tiff/tifftags/planarconfiguration.html
            PlanarConfiguration::Chunky
        } else {
            PlanarConfiguration::Chunky
        };
        Ok(Self {
            new_subfile_type,
            image_width: image_width.expect("image_width not found"),
            image_height: image_height.expect("image_height not found"),
            bits_per_sample: bits_per_sample.expect("bits per sample not found"),
            // Defaults to no compression
            // https://web.archive.org/web/20240329145331/https://www.awaresystems.be/imaging/tiff/tifftags/compression.html
            compression: compression.unwrap_or(CompressionMethod::None),
            photometric_interpretation: photometric_interpretation
                .expect("photometric interpretation not found"),
            document_name,
            image_description,
            strip_offsets,
            orientation,
            samples_per_pixel,
            rows_per_strip,
            strip_byte_counts,
            min_sample_value,
            max_sample_value,
            x_resolution,
            y_resolution,
            planar_configuration,
            resolution_unit,
            software,
            date_time,
            artist,
            host_computer,
            predictor,
            color_map,
            tile_width,
            tile_height,
            tile_offsets,
            tile_byte_counts,
            extra_samples,
            // Uint8 is the default for SampleFormat
            // https://web.archive.org/web/20240329145340/https://www.awaresystems.be/imaging/tiff/tifftags/sampleformat.html
            sample_format: sample_format
                .unwrap_or(vec![SampleFormat::Uint; samples_per_pixel as _]),
            copyright,
            jpeg_tables,
            geo_key_directory,
            model_pixel_scale,
            model_tiepoint,
            other_tags,
            next_ifd_offset,
        })
    }

    /// A general indication of the kind of data contained in this subfile.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/newsubfiletype.html>
    pub fn new_subfile_type(&self) -> Option<u32> {
        self.new_subfile_type
    }

    /// The number of columns in the image, i.e., the number of pixels per row.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/imagewidth.html>
    pub fn image_width(&self) -> u32 {
        self.image_width
    }

    /// The number of rows of pixels in the image.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/imagelength.html>
    pub fn image_height(&self) -> u32 {
        self.image_height
    }

    /// Number of bits per component.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/bitspersample.html>
    pub fn bits_per_sample(&self) -> &[u16] {
        &self.bits_per_sample
    }

    /// Compression scheme used on the image data.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/compression.html>
    pub fn compression(&self) -> CompressionMethod {
        self.compression
    }

    /// The color space of the image data.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/photometricinterpretation.html>
    pub fn photometric_interpretation(&self) -> PhotometricInterpretation {
        self.photometric_interpretation
    }

    /// Document name.
    pub fn document_name(&self) -> Option<&str> {
        self.document_name.as_deref()
    }

    /// A string that describes the subject of the image.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/imagedescription.html>
    pub fn image_description(&self) -> Option<&str> {
        self.image_description.as_deref()
    }

    /// For each strip, the byte offset of that strip.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/stripoffsets.html>
    pub fn strip_offsets(&self) -> Option<&[u64]> {
        self.strip_offsets.as_deref()
    }

    /// The orientation of the image with respect to the rows and columns.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/orientation.html>
    pub fn orientation(&self) -> Option<u16> {
        self.orientation
    }

    /// The number of components per pixel.
    ///
    /// SamplesPerPixel is usually 1 for bilevel, grayscale, and palette-color images.
    /// SamplesPerPixel is usually 3 for RGB images. If this value is higher, ExtraSamples should
    /// give an indication of the meaning of the additional channels.
    pub fn samples_per_pixel(&self) -> u16 {
        self.samples_per_pixel
    }

    /// The number of rows per strip.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/rowsperstrip.html>
    pub fn rows_per_strip(&self) -> Option<u32> {
        self.rows_per_strip
    }

    /// For each strip, the number of bytes in the strip after compression.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/stripbytecounts.html>
    pub fn strip_byte_counts(&self) -> Option<&[u64]> {
        self.strip_byte_counts.as_deref()
    }

    /// The minimum component value used.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/minsamplevalue.html>
    pub fn min_sample_value(&self) -> Option<&[u16]> {
        self.min_sample_value.as_deref()
    }

    /// The maximum component value used.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/maxsamplevalue.html>
    pub fn max_sample_value(&self) -> Option<&[u16]> {
        self.max_sample_value.as_deref()
    }

    /// The number of pixels per ResolutionUnit in the ImageWidth direction.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/xresolution.html>
    pub fn x_resolution(&self) -> Option<f64> {
        self.x_resolution
    }

    /// The number of pixels per ResolutionUnit in the ImageLength direction.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/yresolution.html>
    pub fn y_resolution(&self) -> Option<f64> {
        self.y_resolution
    }

    /// How the components of each pixel are stored.
    ///
    /// The specification defines these values:
    ///
    /// - Chunky format. The component values for each pixel are stored contiguously. For example,
    ///   for RGB data, the data is stored as RGBRGBRGB
    /// - Planar format. The components are stored in separate component planes. For example, RGB
    ///   data is stored with the Red components in one component plane, the Green in another, and
    ///   the Blue in another.
    ///
    /// The specification adds a warning that PlanarConfiguration=2 is not in widespread use and
    /// that Baseline TIFF readers are not required to support it.
    ///
    /// If SamplesPerPixel is 1, PlanarConfiguration is irrelevant, and need not be included.
    ///
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/planarconfiguration.html>
    pub fn planar_configuration(&self) -> PlanarConfiguration {
        self.planar_configuration
    }

    /// The unit of measurement for XResolution and YResolution.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/resolutionunit.html>
    pub fn resolution_unit(&self) -> Option<ResolutionUnit> {
        self.resolution_unit
    }

    /// Name and version number of the software package(s) used to create the image.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/software.html>
    pub fn software(&self) -> Option<&str> {
        self.software.as_deref()
    }

    /// Date and time of image creation.
    ///
    /// The format is: "YYYY:MM:DD HH:MM:SS", with hours like those on a 24-hour clock, and one
    /// space character between the date and the time. The length of the string, including the
    /// terminating NUL, is 20 bytes.
    ///
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/datetime.html>
    pub fn date_time(&self) -> Option<&str> {
        self.date_time.as_deref()
    }

    /// Person who created the image.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/artist.html>
    pub fn artist(&self) -> Option<&str> {
        self.artist.as_deref()
    }

    /// The computer and/or operating system in use at the time of image creation.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/hostcomputer.html>
    pub fn host_computer(&self) -> Option<&str> {
        self.host_computer.as_deref()
    }

    /// A mathematical operator that is applied to the image data before an encoding scheme is
    /// applied.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/predictor.html>
    pub fn predictor(&self) -> Option<Predictor> {
        self.predictor
    }

    /// The tile width in pixels. This is the number of columns in each tile.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/tilewidth.html>
    pub fn tile_width(&self) -> Option<u32> {
        self.tile_width
    }

    /// The tile length (height) in pixels. This is the number of rows in each tile.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/tilelength.html>
    pub fn tile_height(&self) -> Option<u32> {
        self.tile_height
    }

    /// For each tile, the byte offset of that tile, as compressed and stored on disk.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/tileoffsets.html>
    pub fn tile_offsets(&self) -> Option<&[u64]> {
        self.tile_offsets.as_deref()
    }

    /// For each tile, the number of (compressed) bytes in that tile.
    /// <https://web.archive.org/web/20240329145339/https://www.awaresystems.be/imaging/tiff/tifftags/tilebytecounts.html>
    pub fn tile_byte_counts(&self) -> Option<&[u64]> {
        self.tile_byte_counts.as_deref()
    }

    /// Description of extra components.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/extrasamples.html>
    pub fn extra_samples(&self) -> Option<&[u16]> {
        self.extra_samples.as_deref()
    }

    /// Specifies how to interpret each data sample in a pixel.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/sampleformat.html>
    pub fn sample_format(&self) -> &[SampleFormat] {
        &self.sample_format
    }

    /// JPEG quantization and/or Huffman tables.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/jpegtables.html>
    pub fn jpeg_tables(&self) -> Option<&[u8]> {
        self.jpeg_tables.as_deref()
    }

    /// Copyright notice.
    /// <https://web.archive.org/web/20240329145250/https://www.awaresystems.be/imaging/tiff/tifftags/copyright.html>
    pub fn copyright(&self) -> Option<&str> {
        self.copyright.as_deref()
    }

    /// Geospatial tags
    /// <https://web.archive.org/web/20240329145313/https://www.awaresystems.be/imaging/tiff/tifftags/geokeydirectorytag.html>
    pub fn geo_key_directory(&self) -> Option<&GeoKeyDirectory> {
        self.geo_key_directory.as_ref()
    }

    /// Used in interchangeable GeoTIFF files.
    /// <https://web.archive.org/web/20240329145238/https://www.awaresystems.be/imaging/tiff/tifftags/modelpixelscaletag.html>
    pub fn model_pixel_scale(&self) -> Option<&[f64]> {
        self.model_pixel_scale.as_deref()
    }

    /// Used in interchangeable GeoTIFF files.
    /// <https://web.archive.org/web/20240329145303/https://www.awaresystems.be/imaging/tiff/tifftags/modeltiepointtag.html>
    pub fn model_tiepoint(&self) -> Option<&[f64]> {
        self.model_tiepoint.as_deref()
    }

    /// Tags for which the tiff crate doesn't have a hard-coded enum variant.
    pub fn other_tags(&self) -> &HashMap<Tag, Value> {
        &self.other_tags
    }

    /// Construct colormap from colormap tag
    pub fn colormap(&self) -> Option<HashMap<usize, [u8; 3]>> {
        fn cmap_transform(val: u16) -> u8 {
            let val = ((val as f64 / 65535.0) * 255.0).floor();
            if val >= 255.0 {
                255
            } else if val < 0.0 {
                0
            } else {
                val as u8
            }
        }

        if let Some(cmap_data) = &self.color_map {
            let bits_per_sample = self.bits_per_sample[0];
            let count = 2_usize.pow(bits_per_sample as u32);
            let mut result = HashMap::new();

            // TODO: support nodata
            for idx in 0..count {
                let color: [u8; 3] =
                    std::array::from_fn(|i| cmap_transform(cmap_data[idx + i * count]));
                // TODO: Handle nodata value

                result.insert(idx, color);
            }

            Some(result)
        } else {
            None
        }
    }

    fn get_tile_byte_range(&self, x: usize, y: usize) -> Option<Range<u64>> {
        let tile_offsets = self.tile_offsets.as_deref()?;
        let tile_byte_counts = self.tile_byte_counts.as_deref()?;
        let idx = (y * self.tile_count()?.0) + x;
        let offset = tile_offsets[idx] as usize;
        // TODO: aiocogeo has a -1 here, but I think that was in error
        let byte_count = tile_byte_counts[idx] as usize;
        Some(offset as _..(offset + byte_count) as _)
    }

    /// Fetch the tile located at `x` column and `y` row using the provided reader.
    pub async fn fetch_tile(
        &self,
        x: usize,
        y: usize,
        reader: &dyn AsyncFileReader,
    ) -> AsyncTiffResult<Tile> {
        let range = self
            .get_tile_byte_range(x, y)
            .ok_or(AsyncTiffError::General("Not a tiled TIFF".to_string()))?;
        let compressed_bytes = reader.get_image_bytes(range).await?;
        Ok(Tile {
            x,
            y,
            compressed_bytes,
            compression_method: self.compression,
            photometric_interpretation: self.photometric_interpretation,
            jpeg_tables: self.jpeg_tables.clone(),
        })
    }

    /// Fetch the tiles located at `x` column and `y` row using the provided reader.
    pub async fn fetch_tiles(
        &self,
        x: &[usize],
        y: &[usize],
        reader: &dyn AsyncFileReader,
    ) -> AsyncTiffResult<Vec<Tile>> {
        assert_eq!(x.len(), y.len(), "x and y should have same len");

        // 1: Get all the byte ranges for all tiles
        let byte_ranges = x
            .iter()
            .zip(y)
            .map(|(x, y)| {
                self.get_tile_byte_range(*x, *y)
                    .ok_or(AsyncTiffError::General("Not a tiled TIFF".to_string()))
            })
            .collect::<AsyncTiffResult<Vec<_>>>()?;

        // 2: Fetch using `get_image_byte_ranges`
        let buffers = reader.get_image_byte_ranges(byte_ranges).await?;

        // 3: Create tile objects
        let mut tiles = vec![];
        for ((compressed_bytes, &x), &y) in buffers.into_iter().zip(x).zip(y) {
            let tile = Tile {
                x,
                y,
                compressed_bytes,
                compression_method: self.compression,
                photometric_interpretation: self.photometric_interpretation,
                jpeg_tables: self.jpeg_tables.clone(),
            };
            tiles.push(tile);
        }
        Ok(tiles)
    }

    /// Return the number of x/y tiles in the IFD
    /// Returns `None` if this is not a tiled TIFF
    pub fn tile_count(&self) -> Option<(usize, usize)> {
        let x_count = (self.image_width as f64 / self.tile_width? as f64).ceil();
        let y_count = (self.image_height as f64 / self.tile_height? as f64).ceil();
        Some((x_count as usize, y_count as usize))
    }
}

/// Read a single tag from the cursor
async fn read_tag(cursor: &mut AsyncCursor, bigtiff: bool) -> AsyncTiffResult<(Tag, Value)> {
    let tag_name = Tag::from_u16_exhaustive(cursor.read_u16().await?);

    let tag_type_code = cursor.read_u16().await?;
    let tag_type = Type::from_u16(tag_type_code).expect(
        "Unknown tag type {tag_type_code}. TODO: we should skip entries with unknown tag types.",
    );
    let count = if bigtiff {
        cursor.read_u64().await?
    } else {
        cursor.read_u32().await?.into()
    };

    let tag_value = read_tag_value(cursor, tag_type, count, bigtiff).await?;

    Ok((tag_name, tag_value))
}

/// Read a tag's value from the cursor
///
/// NOTE: this does not maintain cursor state
// This is derived from the upstream tiff crate:
// https://github.com/image-rs/image-tiff/blob/6dc7a266d30291db1e706c8133357931f9e2a053/src/decoder/ifd.rs#L369-L639
async fn read_tag_value(
    cursor: &mut AsyncCursor,
    tag_type: Type,
    count: u64,
    bigtiff: bool,
) -> AsyncTiffResult<Value> {
    // Case 0: there are no values so we can return immediately.
    if count == 0 {
        return Ok(Value::List(vec![]));
    }

    let tag_size = match tag_type {
        Type::BYTE | Type::SBYTE | Type::ASCII | Type::UNDEFINED => 1,
        Type::SHORT | Type::SSHORT => 2,
        Type::LONG | Type::SLONG | Type::FLOAT | Type::IFD => 4,
        Type::LONG8
        | Type::SLONG8
        | Type::DOUBLE
        | Type::RATIONAL
        | Type::SRATIONAL
        | Type::IFD8 => 8,
    };

    let value_byte_length = count.checked_mul(tag_size).unwrap();

    // prefetch all tag data
    let mut data = if (bigtiff && value_byte_length <= 8) || value_byte_length <= 4 {
        // value fits in offset field
        let res = cursor.read(value_byte_length).await?;
        if bigtiff {
            cursor.advance(8 - value_byte_length);
        } else {
            cursor.advance(4 - value_byte_length);
        }
        res
    } else {
        // Seek cursor
        let offset = if bigtiff {
            cursor.read_u64().await?
        } else {
            cursor.read_u32().await?.into()
        };
        let reader = cursor
            .reader()
            .get_metadata_bytes(offset..offset + value_byte_length)
            .await?
            .reader();
        EndianAwareReader::new(reader, cursor.endianness())
    };

    // Case 1: there is one value.
    if count == 1 {
        return Ok(match tag_type {
            Type::LONG8 => Value::UnsignedBig(data.read_u64()?),
            Type::SLONG8 => Value::SignedBig(data.read_i64()?),
            Type::DOUBLE => Value::Double(data.read_f64()?),
            Type::RATIONAL => Value::Rational(data.read_u32()?, data.read_u32()?),
            Type::SRATIONAL => Value::SRational(data.read_i32()?, data.read_i32()?),
            Type::IFD8 => Value::IfdBig(data.read_u64()?),
            Type::BYTE | Type::UNDEFINED => Value::Byte(data.read_u8()?),
            Type::SBYTE => Value::Signed(data.read_i8()? as i32),
            Type::SHORT => Value::Short(data.read_u16()?),
            Type::IFD => Value::Ifd(data.read_u32()?),
            Type::SSHORT => Value::Signed(data.read_i16()? as i32),
            Type::LONG => Value::Unsigned(data.read_u32()?),
            Type::SLONG => Value::Signed(data.read_i32()?),
            Type::FLOAT => Value::Float(data.read_f32()?),
            Type::ASCII => {
                if data.as_ref()[0] == 0 {
                    Value::Ascii("".to_string())
                } else {
                    panic!("Invalid tag");
                    // return Err(TiffError::FormatError(TiffFormatError::InvalidTag));
                }
            }
        });
    }

    // Case 2: there is more than one value
    match tag_type {
        Type::BYTE | Type::UNDEFINED => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Byte(data.read_u8()?));
            }
            Ok(Value::List(v))
        }
        Type::SBYTE => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Signed(data.read_i8()? as i32));
            }
            Ok(Value::List(v))
        }
        Type::ASCII => {
            let mut buf = vec![0; count as usize];
            data.read_exact(&mut buf)?;
            if buf.is_ascii() && buf.ends_with(&[0]) {
                let v = std::str::from_utf8(&buf)
                    .map_err(|err| AsyncTiffError::General(err.to_string()))?;
                let v = v.trim_matches(char::from(0));
                Ok(Value::Ascii(v.into()))
            } else {
                panic!("Invalid tag");
                // return Err(TiffError::FormatError(TiffFormatError::InvalidTag));
            }
        }
        Type::SHORT => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Short(data.read_u16()?));
            }
            Ok(Value::List(v))
        }
        Type::SSHORT => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Signed(i32::from(data.read_i16()?)));
            }
            Ok(Value::List(v))
        }
        Type::LONG => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Unsigned(data.read_u32()?));
            }
            Ok(Value::List(v))
        }
        Type::SLONG => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Signed(data.read_i32()?));
            }
            Ok(Value::List(v))
        }
        Type::FLOAT => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Float(data.read_f32()?));
            }
            Ok(Value::List(v))
        }
        Type::DOUBLE => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Double(data.read_f64()?))
            }
            Ok(Value::List(v))
        }
        Type::RATIONAL => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Rational(data.read_u32()?, data.read_u32()?))
            }
            Ok(Value::List(v))
        }
        Type::SRATIONAL => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::SRational(data.read_i32()?, data.read_i32()?))
            }
            Ok(Value::List(v))
        }
        Type::LONG8 => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::UnsignedBig(data.read_u64()?))
            }
            Ok(Value::List(v))
        }
        Type::SLONG8 => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::SignedBig(data.read_i64()?))
            }
            Ok(Value::List(v))
        }
        Type::IFD => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Ifd(data.read_u32()?))
            }
            Ok(Value::List(v))
        }
        Type::IFD8 => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::IfdBig(data.read_u64()?))
            }
            Ok(Value::List(v))
        }
    }
}
