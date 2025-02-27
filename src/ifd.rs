use std::collections::HashMap;
use std::io::Read;
use std::ops::Range;

use bytes::Bytes;
use num_enum::TryFromPrimitive;

use crate::async_reader::AsyncCursor;
use crate::decoder::{decode_tile, DecoderRegistry};
use crate::error::{AiocogeoError, Result};
use crate::geo::{AffineTransform, GeoKeyDirectory, GeoKeyTag};
use crate::tiff::tags::{
    CompressionMethod, PhotometricInterpretation, PlanarConfiguration, Predictor, ResolutionUnit,
    SampleFormat, Tag, Type,
};
use crate::tiff::TiffError;
use crate::tiff::Value;
use crate::AsyncFileReader;

const DOCUMENT_NAME: u16 = 269;

/// A collection of all the IFD
// TODO: maybe separate out the primary/first image IFD out of the vec, as that one should have
// geospatial metadata?
#[derive(Debug)]
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
    ) -> Result<Self> {
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

    pub(crate) jpeg_tables: Option<Vec<u8>>,

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
    async fn read(cursor: &mut AsyncCursor, ifd_start: u64, bigtiff: bool) -> Result<Self> {
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
        dbg!(&tags);

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

    fn from_tags(mut tag_data: HashMap<Tag, Value>, next_ifd_offset: Option<u64>) -> Result<Self> {
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
            dbg!(&tag);
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
                Tag::StripOffsets => {
                    dbg!(&value);
                    strip_offsets = Some(value.into_u64_vec()?)
                }
                Tag::Orientation => orientation = Some(value.into_u16()?),
                Tag::SamplesPerPixel => samples_per_pixel = Some(value.into_u16()?),
                Tag::RowsPerStrip => rows_per_strip = Some(value.into_u32()?),
                Tag::StripByteCounts => {
                    dbg!(&value);
                    strip_byte_counts = Some(value.into_u64_vec()?)
                }
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
                Tag::JPEGTables => jpeg_tables = Some(value.into_u8_vec()?),
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
            dbg!(planar_configuration);
            dbg!(samples_per_pixel);
            println!("planar_configuration not found and samples_per_pixel not 1");
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

    pub fn new_subfile_type(&self) -> Option<u32> {
        self.new_subfile_type
    }

    /// The number of columns in the image, i.e., the number of pixels per row.
    pub fn image_width(&self) -> u32 {
        self.image_width
    }

    /// The number of rows of pixels in the image.
    pub fn image_height(&self) -> u32 {
        self.image_height
    }

    pub fn bits_per_sample(&self) -> &[u16] {
        &self.bits_per_sample
    }

    pub fn compression(&self) -> CompressionMethod {
        self.compression
    }

    pub fn photometric_interpretation(&self) -> PhotometricInterpretation {
        self.photometric_interpretation
    }

    pub fn document_name(&self) -> Option<&str> {
        self.document_name.as_deref()
    }

    pub fn image_description(&self) -> Option<&str> {
        self.image_description.as_deref()
    }

    pub fn strip_offsets(&self) -> Option<&[u64]> {
        self.strip_offsets.as_deref()
    }

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

    pub fn rows_per_strip(&self) -> Option<u32> {
        self.rows_per_strip
    }

    pub fn strip_byte_counts(&self) -> Option<&[u64]> {
        self.strip_byte_counts.as_deref()
    }

    pub fn min_sample_value(&self) -> Option<&[u16]> {
        self.min_sample_value.as_deref()
    }

    pub fn max_sample_value(&self) -> Option<&[u16]> {
        self.max_sample_value.as_deref()
    }

    /// The number of pixels per ResolutionUnit in the ImageWidth direction.
    pub fn x_resolution(&self) -> Option<f64> {
        self.x_resolution
    }

    /// The number of pixels per ResolutionUnit in the ImageLength direction.
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
    pub fn planar_configuration(&self) -> PlanarConfiguration {
        self.planar_configuration
    }

    pub fn resolution_unit(&self) -> Option<ResolutionUnit> {
        self.resolution_unit
    }

    /// Name and version number of the software package(s) used to create the image.
    pub fn software(&self) -> Option<&str> {
        self.software.as_deref()
    }

    /// Date and time of image creation.
    ///
    /// The format is: "YYYY:MM:DD HH:MM:SS", with hours like those on a 24-hour clock, and one
    /// space character between the date and the time. The length of the string, including the
    /// terminating NUL, is 20 bytes.
    pub fn date_time(&self) -> Option<&str> {
        self.date_time.as_deref()
    }

    pub fn artist(&self) -> Option<&str> {
        self.artist.as_deref()
    }

    pub fn host_computer(&self) -> Option<&str> {
        self.host_computer.as_deref()
    }

    pub fn predictor(&self) -> Option<Predictor> {
        self.predictor
    }

    pub fn tile_width(&self) -> Option<u32> {
        self.tile_width
    }
    pub fn tile_height(&self) -> Option<u32> {
        self.tile_height
    }

    pub fn tile_offsets(&self) -> Option<&[u64]> {
        self.tile_offsets.as_deref()
    }
    pub fn tile_byte_counts(&self) -> Option<&[u64]> {
        self.tile_byte_counts.as_deref()
    }

    pub fn extra_samples(&self) -> Option<&[u16]> {
        self.extra_samples.as_deref()
    }

    pub fn sample_format(&self) -> &[SampleFormat] {
        &self.sample_format
    }

    pub fn jpeg_tables(&self) -> Option<&[u8]> {
        self.jpeg_tables.as_deref()
    }

    pub fn copyright(&self) -> Option<&str> {
        self.copyright.as_deref()
    }

    // Geospatial tags
    pub fn geo_key_directory(&self) -> Option<&GeoKeyDirectory> {
        self.geo_key_directory.as_ref()
    }

    pub fn model_pixel_scale(&self) -> Option<&[f64]> {
        self.model_pixel_scale.as_deref()
    }

    pub fn model_tiepoint(&self) -> Option<&[f64]> {
        self.model_tiepoint.as_deref()
    }

    /// Check if an IFD is masked based on a dictionary of tiff tags
    /// https://www.awaresystems.be/imaging/tiff/tifftags/newsubfiletype.html
    /// https://gdal.org/drivers/raster/gtiff.html#internal-nodata-masks
    pub fn is_masked(&self) -> bool {
        if let Some(subfile_type) = self.new_subfile_type {
            (subfile_type == 1 || subfile_type == 2)
                && self.photometric_interpretation == PhotometricInterpretation::TransparencyMask
                && self.compression == CompressionMethod::Deflate
        } else {
            false
        }
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

    /// Returns true if this IFD contains a full resolution image (not an overview)
    pub fn is_full_resolution(&self) -> bool {
        if let Some(val) = self.new_subfile_type {
            val != 0
        } else {
            true
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

    pub async fn get_tile(
        &self,
        x: usize,
        y: usize,
        mut reader: Box<dyn AsyncFileReader>,
        decoder_registry: &DecoderRegistry,
    ) -> Result<Bytes> {
        let range = self
            .get_tile_byte_range(x, y)
            .ok_or(AiocogeoError::General("Not a tiled TIFF".to_string()))?;
        let buf = reader.get_bytes(range).await?;
        decode_tile(
            buf,
            self.photometric_interpretation,
            self.compression,
            self.jpeg_tables.as_deref(),
            decoder_registry,
        )
    }

    pub async fn get_tiles(
        &self,
        x: &[usize],
        y: &[usize],
        mut reader: Box<dyn AsyncFileReader>,
        decoder_registry: &DecoderRegistry,
    ) -> Result<Vec<Bytes>> {
        assert_eq!(x.len(), y.len(), "x and y should have same len");

        // 1: Get all the byte ranges for all tiles
        let byte_ranges = x
            .iter()
            .zip(y)
            .map(|(x, y)| {
                self.get_tile_byte_range(*x, *y)
                    .ok_or(AiocogeoError::General("Not a tiled TIFF".to_string()))
            })
            .collect::<Result<Vec<_>>>()?;

        // 2: Fetch using `get_ranges
        let buffers = reader.get_byte_ranges(byte_ranges).await?;

        // 3: Decode tiles (in the future, separate API)
        let mut decoded_tiles = vec![];
        for buf in buffers {
            let decoded = decode_tile(
                buf,
                self.photometric_interpretation,
                self.compression,
                self.jpeg_tables.as_deref(),
                decoder_registry,
            )?;
            decoded_tiles.push(decoded);
        }
        Ok(decoded_tiles)
    }

    /// Return the number of x/y tiles in the IFD
    /// Returns `None` if this is not a tiled TIFF
    pub fn tile_count(&self) -> Option<(usize, usize)> {
        let x_count = (self.image_width as f64 / self.tile_width? as f64).ceil();
        let y_count = (self.image_height as f64 / self.tile_height? as f64).ceil();
        Some((x_count as usize, y_count as usize))
    }

    /// Return the geotransform of the image
    ///
    /// This does not yet implement decimation
    pub fn geotransform(&self) -> Option<AffineTransform> {
        if let (Some(model_pixel_scale), Some(model_tiepoint)) =
            (&self.model_pixel_scale, &self.model_tiepoint)
        {
            Some(AffineTransform::new(
                model_pixel_scale[0],
                0.0,
                model_tiepoint[3],
                0.0,
                -model_pixel_scale[1],
                model_tiepoint[4],
            ))
        } else {
            None
        }
    }

    /// Return the bounds of the image in native crs
    pub fn native_bounds(&self) -> Option<(f64, f64, f64, f64)> {
        if let Some(gt) = self.geotransform() {
            let tlx = gt.c();
            let tly = gt.f();

            let brx = tlx + (gt.a() * self.image_width as f64);
            let bry = tly + (gt.e() * self.image_height as f64);
            Some((tlx, bry, brx, tly))
        } else {
            None
        }
    }
}

/// Read a single tag from the cursor
async fn read_tag(cursor: &mut AsyncCursor, bigtiff: bool) -> Result<(Tag, Value)> {
    let start_cursor_position = cursor.position();

    let tag_name = Tag::from_u16_exhaustive(cursor.read_u16().await?);

    let tag_type_code = cursor.read_u16().await?;
    let tag_type = Type::from_u16(tag_type_code).expect(
        "Unknown tag type {tag_type_code}. TODO: we should skip entries with unknown tag types.",
    );
    dbg!(tag_name, tag_type);
    let count = if bigtiff {
        cursor.read_u64().await?
    } else {
        cursor.read_u32().await?.into()
    };

    let tag_value = read_tag_value(cursor, tag_type, count, bigtiff).await?;

    // TODO: better handle management of cursor state
    let ifd_entry_size = if bigtiff { 20 } else { 12 };
    cursor.seek(start_cursor_position + ifd_entry_size);

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
) -> Result<Value> {
    // Case 1: there are no values so we can return immediately.
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

    // Case 2: there is one value.
    if count == 1 {
        // 2a: the value is 5-8 bytes and we're in BigTiff mode.
        dbg!("case 2a");
        if bigtiff && value_byte_length > 4 && value_byte_length <= 8 {
            let mut data = cursor.read(value_byte_length).await?;

            return Ok(match tag_type {
                Type::LONG8 => Value::UnsignedBig(data.read_u64()?),
                Type::SLONG8 => Value::SignedBig(data.read_i64()?),
                Type::DOUBLE => Value::Double(data.read_f64()?),
                Type::RATIONAL => Value::Rational(data.read_u32()?, data.read_u32()?),
                Type::SRATIONAL => Value::SRational(data.read_i32()?, data.read_i32()?),
                Type::IFD8 => Value::IfdBig(data.read_u64()?),
                Type::BYTE
                | Type::SBYTE
                | Type::ASCII
                | Type::UNDEFINED
                | Type::SHORT
                | Type::SSHORT
                | Type::LONG
                | Type::SLONG
                | Type::FLOAT
                | Type::IFD => unreachable!(),
            });
        }

        // NOTE: we should only be reading value_byte_length when it's 4 bytes or fewer. Right now
        // we're reading even if it's 8 bytes, but then only using the first 4 bytes of this
        // buffer.
        let mut data = cursor.read(value_byte_length).await?;

        // 2b: the value is at most 4 bytes or doesn't fit in the offset field.
        dbg!("case 2b");
        return Ok(match tag_type {
            Type::BYTE | Type::UNDEFINED => Value::Byte(data.read_u8()?),
            Type::SBYTE => Value::Signed(data.read_i8()? as i32),
            Type::SHORT => Value::Short(data.read_u16()?),
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
            Type::LONG8 => {
                let offset = data.read_u32()?;
                cursor.seek(offset as _);
                Value::UnsignedBig(cursor.read_u64().await?)
            }
            Type::SLONG8 => {
                let offset = data.read_u32()?;
                cursor.seek(offset as _);
                Value::SignedBig(cursor.read_i64().await?)
            }
            Type::DOUBLE => {
                let offset = data.read_u32()?;
                cursor.seek(offset as _);
                Value::Double(cursor.read_f64().await?)
            }
            Type::RATIONAL => {
                let offset = data.read_u32()?;
                cursor.seek(offset as _);
                let numerator = cursor.read_u32().await?;
                let denominator = cursor.read_u32().await?;
                Value::Rational(numerator, denominator)
            }
            Type::SRATIONAL => {
                let offset = data.read_u32()?;
                cursor.seek(offset as _);
                let numerator = cursor.read_i32().await?;
                let denominator = cursor.read_i32().await?;
                Value::SRational(numerator, denominator)
            }
            Type::IFD => Value::Ifd(data.read_u32()?),
            Type::IFD8 => {
                let offset = data.read_u32()?;
                cursor.seek(offset as _);
                Value::IfdBig(cursor.read_u64().await?)
            }
        });
    }

    // Case 3: There is more than one value, but it fits in the offset field.
    if value_byte_length <= 4 || bigtiff && value_byte_length <= 8 {
        dbg!("case 3");
        let mut data = cursor.read(value_byte_length).await?;
        if bigtiff {
            cursor.advance(8 - value_byte_length);
        } else {
            cursor.advance(4 - value_byte_length);
        }

        match tag_type {
            Type::BYTE | Type::UNDEFINED => {
                return {
                    Ok(Value::List(
                        (0..count)
                            .map(|_| Value::Byte(data.read_u8().unwrap()))
                            .collect(),
                    ))
                };
            }
            Type::SBYTE => {
                return {
                    Ok(Value::List(
                        (0..count)
                            .map(|_| Value::Signed(data.read_i8().unwrap() as i32))
                            .collect(),
                    ))
                }
            }
            Type::ASCII => {
                let mut buf = vec![0; count as usize];
                data.read_exact(&mut buf)?;
                if buf.is_ascii() && buf.ends_with(&[0]) {
                    let v = std::str::from_utf8(&buf)
                        .map_err(|err| AiocogeoError::General(err.to_string()))?;
                    let v = v.trim_matches(char::from(0));
                    return Ok(Value::Ascii(v.into()));
                } else {
                    panic!("Invalid tag");
                    // return Err(TiffError::FormatError(TiffFormatError::InvalidTag));
                }
            }
            Type::SHORT => {
                let mut v = Vec::new();
                for _ in 0..count {
                    v.push(Value::Short(data.read_u16()?));
                }
                return Ok(Value::List(v));
            }
            Type::SSHORT => {
                let mut v = Vec::new();
                for _ in 0..count {
                    v.push(Value::Signed(i32::from(data.read_i16()?)));
                }
                return Ok(Value::List(v));
            }
            Type::LONG => {
                let mut v = Vec::new();
                for _ in 0..count {
                    v.push(Value::Unsigned(data.read_u32()?));
                }
                return Ok(Value::List(v));
            }
            Type::SLONG => {
                let mut v = Vec::new();
                for _ in 0..count {
                    v.push(Value::Signed(data.read_i32()?));
                }
                return Ok(Value::List(v));
            }
            Type::FLOAT => {
                let mut v = Vec::new();
                for _ in 0..count {
                    v.push(Value::Float(data.read_f32()?));
                }
                return Ok(Value::List(v));
            }
            Type::IFD => {
                let mut v = Vec::new();
                for _ in 0..count {
                    v.push(Value::Ifd(data.read_u32()?));
                }
                return Ok(Value::List(v));
            }
            Type::LONG8
            | Type::SLONG8
            | Type::RATIONAL
            | Type::SRATIONAL
            | Type::DOUBLE
            | Type::IFD8 => {
                unreachable!()
            }
        }
    }

    // Seek cursor
    let offset = if bigtiff {
        cursor.read_u64().await?
    } else {
        cursor.read_u32().await?.into()
    };
    cursor.seek(offset);

    // Case 4: there is more than one value, and it doesn't fit in the offset field.
    dbg!("case 4");
    match tag_type {
        // TODO check if this could give wrong results
        // at a different endianess of file/computer.
        Type::BYTE | Type::UNDEFINED => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Byte(cursor.read_u8().await?))
            }
            Ok(Value::List(v))
        }
        Type::SBYTE => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Signed(cursor.read_i8().await? as i32))
            }
            Ok(Value::List(v))
        }
        Type::SHORT => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Short(cursor.read_u16().await?))
            }
            Ok(Value::List(v))
        }
        Type::SSHORT => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Signed(cursor.read_i16().await? as i32))
            }
            Ok(Value::List(v))
        }
        Type::LONG => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Unsigned(cursor.read_u32().await?))
            }
            Ok(Value::List(v))
        }
        Type::SLONG => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Signed(cursor.read_i32().await?))
            }
            Ok(Value::List(v))
        }
        Type::FLOAT => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Float(cursor.read_f32().await?))
            }
            Ok(Value::List(v))
        }
        Type::DOUBLE => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Double(cursor.read_f64().await?))
            }
            Ok(Value::List(v))
        }
        Type::RATIONAL => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Rational(
                    cursor.read_u32().await?,
                    cursor.read_u32().await?,
                ))
            }
            Ok(Value::List(v))
        }
        Type::SRATIONAL => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::SRational(
                    cursor.read_i32().await?,
                    cursor.read_i32().await?,
                ))
            }
            Ok(Value::List(v))
        }
        Type::LONG8 => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::UnsignedBig(cursor.read_u64().await?))
            }
            Ok(Value::List(v))
        }
        Type::SLONG8 => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::SignedBig(cursor.read_i64().await?))
            }
            Ok(Value::List(v))
        }
        Type::IFD => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::Ifd(cursor.read_u32().await?))
            }
            Ok(Value::List(v))
        }
        Type::IFD8 => {
            let mut v = Vec::with_capacity(count as _);
            for _ in 0..count {
                v.push(Value::IfdBig(cursor.read_u64().await?))
            }
            Ok(Value::List(v))
        }
        Type::ASCII => {
            let mut out = vec![0; count as _];
            let mut buf = cursor.read(count).await?;
            buf.read_exact(&mut out)?;

            // Strings may be null-terminated, so we trim anything downstream of the null byte
            if let Some(first) = out.iter().position(|&b| b == 0) {
                out.truncate(first);
            }
            Ok(Value::Ascii(
                String::from_utf8(out).map_err(|err| AiocogeoError::General(err.to_string()))?,
            ))
        }
    }
}
