# async-tiff

An async, low-level [TIFF](https://en.wikipedia.org/wiki/TIFF) reader for Rust and Python.

- [**Rust documentation**](https://docs.rs/async-tiff/)
- [**Python documentation**](https://developmentseed.org/async-tiff/latest/)
    - For a higher-level Python API to read GeoTIFF files, see [`async-geotiff`][async-geotiff].

[async-geotiff]: https://developmentseed.org/async-geotiff/latest/

## Features

- Async, read-only support for tiled TIFF images.
- Read directly from object storage providers, via the `object_store` crate.
- Separation of concerns between data reading and decoding so that IO-bound and CPU-bound tasks can be scheduled appropriately.
- Support for user-defined decompression algorithms.
- Tile request merging and concurrency.
- Integration with the [`ndarray`](https://crates.io/crates/ndarray) crate for easy manipulation of decoded image data.
- Support for GeoTIFF tag metadata.

## Example

```rust
# tokio_test::block_on(async {
# use std::sync::Arc;
# use std::env::current_dir;
#
use object_store::local::LocalFileSystem;
use async_tiff::metadata::TiffMetadataReader;
use async_tiff::metadata::cache::ReadaheadMetadataCache;
use async_tiff::reader::ObjectReader;
use async_tiff::TIFF;

let store = Arc::new(LocalFileSystem::new_with_prefix(current_dir().unwrap()).unwrap());
let reader = ObjectReader::new(store, "fixtures/image-tiff/tiled-jpeg-rgb-u8.tif".into());
let cache = ReadaheadMetadataCache::new(reader.clone());

let mut meta = TiffMetadataReader::try_open(&cache).await.unwrap();
let ifds = meta.read_all_ifds(&cache).await.unwrap();
let tiff = TIFF::new(ifds, meta.endianness());

// Fetch and decode a tile
let tile = tiff.ifds()[0].fetch_tile(0, 0, &reader).await.unwrap();
let array = tile.decode(&Default::default()).unwrap();
println!("shape: {:?}, dtype: {:?}", array.shape(), array.data_type());
# })
```

## Background

The existing [`tiff` crate](https://crates.io/crates/tiff) is great, but only supports synchronous reading of TIFF files. Furthermore, due to low maintenance bandwidth it is not designed for extensibility (see [#250](https://github.com/image-rs/image-tiff/issues/250)).

This crate was initially forked from the `tiff` crate, and still maintains some of its TIFF tag parsing code.
