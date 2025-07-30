use crate::error::{AsyncTiffError, AsyncTiffResult};
use crate::geo::{AffineTransform, GeoKeyDirectory, GeoKeyTag};
use crate::tiff::tags::Tag;
use crate::tiff::Value;
use futures::stream::Once;
use num_enum::TryFromPrimitive;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::sync::{Arc, LazyLock, OnceLock, RwLock};

/// Trait to implement for custom tags, such as Geo, EXIF, OME, etc
/// your type should also implement `Clone`
// Send + Sync are required for Python, where `dyn ExtraTags` needs `Send` and `Sync`
pub trait ExtraTags: ExtraTagsBlankets + std::any::Any + Debug + Send + Sync {
    /// a list of tags this entry processes
    /// e.g. for Geo this would be [34735, 34736, 34737]
    fn tags(&self) -> &'static [Tag];
    /// process a single tag, use interior mutability if needed
    fn process_tag(&self, tag: Tag, value: Value) -> AsyncTiffResult<()>;

}

// we need to do a little dance to do an object-safe deep clone
// https://stackoverflow.com/a/30353928/14681457
// also blanket impls for downcasting to any
pub trait ExtraTagsBlankets {
    fn clone_arc(&self) -> Arc<dyn ExtraTags>;
    fn as_any_arc(self: Arc<Self>) -> Arc<dyn std::any::Any + Send + Sync>;
}

impl<T> ExtraTagsBlankets for T
where
    T: 'static + ExtraTags + Clone,
{
    fn clone_arc(&self) -> Arc<dyn ExtraTags> {
        Arc::new(self.clone())
    }

    fn as_any_arc(self: Arc<Self>) -> Arc<dyn std::any::Any + Send + Sync> {
        self
    }
}

/// The registry in which extra tags (parsers) are registered
/// This is passed to TODO
#[derive(Debug, Clone)]
pub struct ExtraTagsRegistry(HashMap<Tag, Arc<dyn ExtraTags>>);

impl ExtraTagsRegistry {
    /// Create a new, empty `ExtraTagsRegistry`
    pub fn new() -> Self {
        Self(HashMap::new())
    }
    /// Register an ExtraTags so their tags are parsed and stored in the ifd's `extra_tags``
    pub fn register(&mut self, tags: Arc<dyn ExtraTags>) -> AsyncTiffResult<()> {
        // check for duplicates
        for tag in tags.tags() {
            if self.0.contains_key(tag) {
                return Err(AsyncTiffError::General(format!(
                    "Tag {tag:?} already registered in {self:?}!"
                )));
            }
        }
        // add to self
        for tag in tags.tags() {
            self.0.insert(*tag, tags.clone());
        }
        Ok(())
    }

    /// deep clone so we have different registries per IFD
    pub(crate) fn deep_clone(&self) -> Self {
        let mut new_registry = ExtraTagsRegistry::new();

        // we need to do some magic, because we can have multiple tags pointing to the same arc
        let mut seen = HashSet::new();
        for extra_tags in self.0.values() {
            // only add if this is the first encountered reference to this arc
            // (using thin pointer equality: https://stackoverflow.com/a/67114787/14681457 ; https://github.com/rust-lang/rust/issues/46139#issuecomment-346971153)
            if seen.insert(Arc::as_ptr(extra_tags) as *const ()) {
                if let Err(e) = new_registry.register(extra_tags.clone_arc()) {
                    panic!("{e}");
                }
            }
        }

        new_registry
    }
}

impl Default for ExtraTagsRegistry {
    fn default() -> Self {
        let mut new = Self::new();
        new.register(Arc::new(GeoTags::default())).unwrap();
        new
    }
}

#[derive(Debug, Clone, Default)]
pub struct GeoTags {
    // we use a bunch of `OnceLock`s here, because the alternative would be a
    // state machine with an `RwLock` which isn't Clone
    model_tiepoint: OnceLock<Vec<f64>>,
    model_pixel_scale: OnceLock<[f64; 3]>,
    model_transform: OnceLock<[f64; 16]>,
    geo_dir: OnceLock<GeoKeyDirectory>,
    geo_dir_data: OnceLock<Vec<u16>>,
    geo_ascii_params: OnceLock<String>,
    geo_double_params: OnceLock<Vec<f64>>,
}

impl GeoTags {
    pub fn affine(&self) -> Option<AffineTransform> {
        if let Some(transform) = self.model_transform.get() {
            todo!("implement https://docs.ogc.org/is/19-008r4/19-008r4.html#_geotiff_tags_for_coordinate_transformations")
        } else if let (Some(model_tiepoint), Some(model_pixel_scale)) = (self.model_tiepoint.get(), self.model_pixel_scale.get()) {
            Some(AffineTransform::new(model_pixel_scale[0], 0.0, model_tiepoint[3], 0.0, -model_pixel_scale[1], model_tiepoint[4]))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub enum MaybePartialGeoKeyDirectory {
    Partial {
        geo_key_directory_data: OnceLock<Vec<u16>>,
        geo_ascii_params: OnceLock<String>,
        geo_double_params: OnceLock<Vec<f64>>,
    },
    Parsed(GeoKeyDirectory),
}

impl Default for MaybePartialGeoKeyDirectory {
    fn default() -> Self {
        Self::Partial {
            geo_key_directory_data: OnceLock::new(),
            geo_ascii_params: OnceLock::new(),
            geo_double_params: OnceLock::new(),
        }
    }
}

impl MaybePartialGeoKeyDirectory {
    /// maybe parse self, returns `Ok(true)` if parsed this call
    fn maybe_parse(&mut self) -> AsyncTiffResult<bool> {
        match self {
            Self::Partial {
                geo_key_directory_data,
                geo_ascii_params,
                geo_double_params,
            } if geo_key_directory_data.get().is_some()
                && geo_ascii_params.get().is_some()
                && geo_double_params.get().is_some() =>
            {
                // take out the data so we can mutate self
                let data = geo_key_directory_data.take().unwrap();
                let geo_ascii_params = geo_ascii_params.take().unwrap();
                let geo_double_params = geo_double_params.take().unwrap();

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
                    let tag_name = GeoKeyTag::try_from_primitive(key_id)
                        .expect("Unknown GeoKeyTag id: {key_id}");

                    let tag_location = chunk[1];
                    let count = chunk[2];
                    let value_offset = chunk[3];

                    if tag_location == 0 {
                        tags.insert(tag_name, Value::Short(value_offset));
                    } else if Tag::from_u16_exhaustive(tag_location) == Tag::GeoAsciiParamsTag {
                        // If the tag_location points to the value of
                        // Tag::GeoAsciiParamsTag, then we need to extract a
                        // subslice from GeoAsciiParamsTag
                        let value_offset = value_offset as usize;
                        let mut s = &geo_ascii_params[value_offset..value_offset + count as usize];

                        // It seems that this string subslice might always
                        // include the final | character?
                        if s.ends_with('|') {
                            s = &s[0..s.len() - 1];
                        }

                        tags.insert(tag_name, Value::Ascii(s.to_string()));
                    } else if Tag::from_u16_exhaustive(tag_location) == Tag::GeoDoubleParamsTag {
                        // If the tag_location points to the value of
                        // Tag::GeoDoubleParamsTag, then we need to extract a
                        // subslice from GeoDoubleParamsTag
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
                *self = MaybePartialGeoKeyDirectory::Parsed(GeoKeyDirectory::from_tags(tags)?);
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

static GEO_TAGS: LazyLock<Vec<Tag>> = LazyLock::new(|| {
    vec![
        Tag::ModelTiepointTag,
        Tag::ModelPixelScaleTag,
        Tag::ModelTransformationTag,
        Tag::GeoKeyDirectoryTag,
        Tag::GeoAsciiParamsTag,
        Tag::GeoDoubleParamsTag,
    ]
});

impl ExtraTags for GeoTags {
    fn tags(&self) -> &'static [Tag] {
        &GEO_TAGS
    }

    fn process_tag(&self, tag: Tag, value: Value) -> AsyncTiffResult<()> {
        match tag {
            Tag::ModelTiepointTag => {
                // https://docs.ogc.org/is/19-008r4/19-008r4.html#_requirements_class_modeltiepointtag
                self.model_tiepoint.set(value.into_f64_vec()?).unwrap()
            }
            Tag::ModelPixelScaleTag => {
                // unwrapping on non-spec compliance?
                // https://docs.ogc.org/is/19-008r4/19-008r4.html#_requirements_class_modelpixelscaletag
                self.model_pixel_scale.set(value.into_f64_vec()?.try_into().unwrap()).unwrap()
            }
            Tag::ModelTransformationTag => {
                // unwrapping on non-spec compliance?
                // https://docs.ogc.org/is/19-008r4/19-008r4.html#_requirements_class_modeltransformationtag
                self.model_transform.set(value.into_f64_vec()?.try_into().unwrap()).unwrap()
            }
            Tag::GeoKeyDirectoryTag => {
                self.geo_dir_data.set(value.into_u16_vec()?).unwrap();
            }
            Tag::GeoAsciiParamsTag => {
                self.geo_ascii_params.set(value.into_string()?).unwrap();
            }
            Tag::GeoDoubleParamsTag => {
                self.geo_double_params.set(value.into_f64_vec()?).unwrap();
            }
            _ => unreachable!(),
        }
        // self.geo_dir.maybe_parse()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::LazyLock;

    #[derive(Debug, Clone, PartialEq)]
    struct TestyTag;

    static TESTY_TAGS: LazyLock<Vec<Tag>> = LazyLock::new(|| {
        vec![
            Tag::from_u16_exhaustive(u16::MAX),
            Tag::from_u16_exhaustive(u16::MAX - 1),
        ]
    });

    impl ExtraTags for TestyTag {
        fn tags(&self) -> &'static [Tag] {
            &TESTY_TAGS
        }

        fn process_tag(
            &self,
            tag: Tag,
            value: crate::tiff::Value,
        ) -> crate::error::AsyncTiffResult<()> {
            println!("received {tag:?}: {value:?}");
            Ok(())
        }
    }

    #[test]
    fn test_register() {
        let mut registry = ExtraTagsRegistry::new();
        assert!(registry.0.is_empty());
        let a1: Arc<dyn ExtraTags> = Arc::new(TestyTag);
        registry.register(a1.clone()).unwrap();
        assert_eq!(registry.0.len(), TestyTag.tags().len());
        for tag in a1.tags() {
            // very strict equality check
            assert!(Arc::ptr_eq(&registry.0[tag], &a1));
        }
    }

    #[test]
    fn test_overlap_err() {
        let mut registry = ExtraTagsRegistry::new();
        assert!(registry.0.is_empty());
        registry.register(Arc::new(TestyTag)).unwrap();
        assert!(matches!(
            registry.register(Arc::new(TestyTag)).unwrap_err(),
            AsyncTiffError::General(_)
        ));
    }

    #[test]
    fn test_deep_clone() {
        let mut registry = ExtraTagsRegistry::new();
        let a1: Arc<dyn ExtraTags> = Arc::new(TestyTag);
        registry.register(a1.clone()).unwrap();
        let r2 = registry.deep_clone();
        for tags in a1.tags().windows(2) {
            // all should refer to the same Arc
            assert!(Arc::ptr_eq(&r2.0[&tags[0]], &r2.0[&tags[1]]));
            // which is different from the previous
            assert!(!Arc::ptr_eq(&a1, &r2.0[&tags[0]]));
            assert!(!Arc::ptr_eq(&a1, &r2.0[&tags[1]]));
        }
    }

    #[test]
    fn test_geo() {
        let registry = ExtraTagsRegistry::default();
        // create a sample hashmap
        let hmap = HashMap::from([(
            Tag::ModelTiepointTag,
            Value::List([0.0, 0.0, 0.0, 350807.4, 5316081.3, 0.0f64].map(|v| Value::Double(v)).into()),
        ),(
            Tag::ModelPixelScaleTag,
            Value::List([100.0, 100.0, 0.0f64].map(|v| Value::Double(v)).into())
        ),(
            Tag::GeoKeyDirectoryTag,
            Value::List([
                 1,     0,  2,     4,
                  1024,     0,  1,     1,
                  1025,     0,  1,     1,
                  3072,     0,  1, 32660,
                  3073, 34737, 25,     0
            ].map(|v| Value::Unsigned(v)).into())
        ),(
            Tag::GeoAsciiParamsTag,
            Value::Ascii("UTM Zone 60 N with WGS 84|".into())
        )
        ]);
        for (k,v) in hmap {
            if let Some(ref entry) = registry.0.get(&k) {
                entry.process_tag(k, v).unwrap();
            }
        }
        let geo_dir = registry.0[&GEO_TAGS[0]].clone().as_any_arc().downcast::<GeoTags>().unwrap();
    }
}
