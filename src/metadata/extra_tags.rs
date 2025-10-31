//! # Register parsers for additional tags
//!
//! Simplified example for exif tags parser
//!
//!  ```
//! # use std::sync::{LazyLock, OnceLock, Arc};
//! # use std::env::current_dir;
//! # use async_tiff::tiff::{Value, tags::Tag};
//! # use async_tiff::error::AsyncTiffResult;
//! # use async_tiff::reader::{ObjectReader, AsyncFileReader};
//! # use async_tiff::metadata::TiffMetadataReader;
//! use async_tiff::metadata::extra_tags::{ExtraTags, ExtraTagsRegistry};
//! # use object_store::local::LocalFileSystem;
//! // see https://www.media.mit.edu/pia/Research/deepview/exif.html#ExifTags
//! // or exif spec: https://www.cipa.jp/std/documents/download_e.html?DC-008-Translation-2023-E
//! // / all tags processed by your extension
//! pub const EXIF_TAGS: [Tag; 3] = [
//!     Tag::Unknown(34665), // Exif IFD pointer
//!     Tag::Unknown(34853), // GPS IFD pointer
//!     Tag::Unknown(40965), // Interoperability IFD pointer
//! ];
//!
//! // / the struct that stores the data (using interior mutability)
//! #[derive(Debug, Clone, Default)]
//! pub struct ExifTags {
//!     pub exif: OnceLock<u32>,
//!     pub gps: OnceLock<u32>,
//!     pub interop: OnceLock<u32>,
//!     // would also hold e.g. a TiffMetadataReader to read exif IFDs
//! }
//!
//! impl ExtraTags for ExifTags {
//!     fn tags(&self) -> &'static [Tag] {
//!          &EXIF_TAGS
//!     }
//!
//!     fn process_tag(&self, tag:Tag, value: Value) -> AsyncTiffResult<()> {
//!         match tag {
//!             Tag::Unknown(34665) => self.exif.set(value.into_u32()?).unwrap(),
//!             Tag::Unknown(34853) => self.gps.set(value.into_u32()?).unwrap(),
//!             Tag::Unknown(40965) => self.interop.set(value.into_u32()?).unwrap(),
//!             _ => {}
//!         }
//!         Ok(())
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     // create an empty registry
//!     let mut registry = ExtraTagsRegistry::new();
//!     // register our custom extra tags
//!     registry.register(Arc::new(ExifTags::default()));
//!
//!     let store = Arc::new(LocalFileSystem::new_with_prefix(current_dir().unwrap()).unwrap());
//!     let path = "tests/sample-exif.tiff";
//!     let reader =
//!         Arc::new(ObjectReader::new(store.clone(), path.into())) as Arc<dyn AsyncFileReader>;
//!     let mut metadata_reader = TiffMetadataReader::try_open(&reader).await.unwrap();
//!     // get the first ifd
//!     let ifd = &metadata_reader
//!         .read_all_ifds(&reader, registry)
//!         .await
//!         .unwrap()[0];
//!
//!     // access by any of our registered tags
//!     let exif = ifd.extra_tags()[&EXIF_TAGS[0]]
//!         .clone()
//!         .as_any_arc()
//!         .downcast::<ExifTags>()
//!         .unwrap();
//!     assert!(exif.exif.get().is_some());
//!     assert!(exif.gps.get().is_some());
//!     // our image doesn't have interop info
//!     assert!(exif.interop.get().is_none());
//! }
//! ```

use crate::error::{AsyncTiffError, AsyncTiffResult};
use crate::tiff::tags::Tag;
use crate::tiff::Value;
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::ops::Index;
use std::sync::Arc;

/// Trait to implement for custom tags, such as Geo, EXIF, OME, etc
///
/// your type should also implement `Clone` for blanket implementations of [`ExtraTagsBlankets`]
///
/// ```
/// # use async_tiff::tiff::{Value, tags::Tag};
/// # use async_tiff::error::AsyncTiffResult;
/// use async_tiff::metadata::extra_tags::ExtraTags;
/// # use std::sync::OnceLock;
///
/// pub const EXIF_TAGS: [Tag; 3] = [
///     Tag::Unknown(34665), // Exif IFD pointer
///     Tag::Unknown(34853), // GPS IFD pointer
///     Tag::Unknown(40965), // Interoperability IFD pointer
/// ];
///
/// // / the struct that stores the data (using interior mutability)
/// #[derive(Debug, Clone, Default)]
/// pub struct ExifTags {
///     pub exif: OnceLock<u32>,
///     pub gps: OnceLock<u32>,
///     pub interop: OnceLock<u32>,
///     // would also hold e.g. a TiffMetadataReader to read exif IFDs
/// }
///
/// impl ExtraTags for ExifTags {
///     fn tags(&self) -> &'static [Tag] {
///          &EXIF_TAGS
///     }
///
///     fn process_tag(&self, tag:Tag, value: Value) -> AsyncTiffResult<()> {
///         match tag {
///             Tag::Unknown(34665) => self.exif.set(value.into_u32()?).unwrap(),
///             Tag::Unknown(34853) => self.gps.set(value.into_u32()?).unwrap(),
///             Tag::Unknown(40965) => self.interop.set(value.into_u32()?).unwrap(),
///             _ => {}
///         }
///         Ok(())
///     }
/// }
/// ```
// Send + Sync are required for Python, where `dyn ExtraTags` needs `Send` and `Sync`
pub trait ExtraTags: ExtraTagsBlankets + Any + Debug + Send + Sync {
    /// a list of tags this entry processes
    ///
    /// e.g. for Geo this would be [34735, 34736, 34737]
    fn tags(&self) -> &'static [Tag];
    /// process a single tag, using internal mutability if needed
    fn process_tag(&self, tag: Tag, value: Value) -> AsyncTiffResult<()>;
}

/// Extra trait with blanket implementations for object-safe cloning and casting
///
/// Automatically implemented if your type implements [`ExtraTags`] and [`Clone`]
///
/// ```
/// # use std::sync::Arc;
/// # use async_tiff::tiff::{Value, tags::Tag};
/// # use async_tiff::error::AsyncTiffResult;
/// use async_tiff::metadata::extra_tags::ExtraTags;
/// // derive these
/// #[derive(Debug, Clone)]
/// pub struct MyTags;
///
/// // custom functionality
/// impl MyTags {
///     fn forty_two(&self) -> u32 {42}
/// }
///
/// // implement ExtraTags
/// impl ExtraTags for MyTags {
///     fn tags(&self) -> &'static [Tag] {
///         &[]
///     }
///
///     fn process_tag(&self, _tag:Tag, _value:Value) -> AsyncTiffResult<()> {
///         Ok(())
///     }
/// }
///
/// fn main() {
///     // allows for deep cloning
///     let my_tags = Arc::new(MyTags) as Arc<dyn ExtraTags>;
///     let other_my_tags = my_tags.clone_arc();
///     assert!(Arc::ptr_eq(&my_tags, &my_tags.clone()));
///     assert!(!Arc::ptr_eq(&my_tags, &other_my_tags));
///
///     // and downcasting
///     let my_tags_concrete = my_tags.as_any_arc().downcast::<MyTags>().unwrap();
///     assert_eq!(my_tags_concrete.forty_two(), 42);
/// }
/// ```
///
/// This works since blanket implementations are done on concrete types and only
/// their signatures (function pointer) will end up in the vtable
/// <https://stackoverflow.com/a/30353928/14681457>
pub trait ExtraTagsBlankets {
    /// deep clone
    fn clone_arc(&self) -> Arc<dyn ExtraTags>;
    /// convert to any for downcasting
    fn as_any_arc(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
}

impl<T> ExtraTagsBlankets for T
where
    T: 'static + ExtraTags + Clone,
{
    fn clone_arc(&self) -> Arc<dyn ExtraTags> {
        Arc::new(self.clone())
    }

    fn as_any_arc(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
}

/// The registry in which extra tags (parsers) are registered
///
/// Pass this to [`crate::metadata::TiffMetadataReader`] when reading.
///
/// ```
/// # use async_tiff::reader::{AsyncFileReader, ObjectReader};
/// # use async_tiff::metadata::TiffMetadataReader;
/// use async_tiff::metadata::extra_tags::ExtraTagsRegistry;
/// # use std::sync::Arc;
/// # use std::env::current_dir;
/// # use object_store::local::LocalFileSystem;
///
/// #[tokio::main]
/// async fn main() {
///     let registry = ExtraTagsRegistry::default();
///
///     let store = Arc::new(LocalFileSystem::new_with_prefix(current_dir().unwrap()).unwrap());
///     # let path = "tests/sample-exif.tiff";
///     let reader =
///         Arc::new(ObjectReader::new(store.clone(), path.into())) as Arc<dyn AsyncFileReader>;
///     let mut metadata_reader = TiffMetadataReader::try_open(&reader).await.unwrap();
///     // get first ifd
///     let ifd = &metadata_reader
///         .read_all_ifds(&reader, registry)
///         .await
///         .unwrap()[0];
///     // retrieve the registry
///     println!("{:?}",ifd.extra_tags());
/// }
/// ```
///
#[derive(Debug, Clone)]
pub struct ExtraTagsRegistry(HashMap<Tag, Arc<dyn ExtraTags>>);

impl ExtraTagsRegistry {
    /// Create a new, empty `ExtraTagsRegistry`
    pub fn new() -> Self {
        Self(HashMap::new())
    }
    /// checks if we have an entry for this tag
    pub fn contains(&self, tag: &Tag) -> bool {
        self.0.contains_key(tag)
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
        Self::new() // add e.g. geo tags later
    }
}

impl Index<&Tag> for ExtraTagsRegistry {
    type Output = Arc<dyn ExtraTags>;
    fn index(&self, index: &Tag) -> &Self::Output {
        &self.0[index]
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
}
