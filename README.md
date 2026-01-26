# async-tiff

An async, low-level [TIFF](https://en.wikipedia.org/wiki/TIFF) reader for Rust and Python.

[**Rust documentation**](https://docs.rs/async-tiff/) <br/>
[**Python documentation**](https://developmentseed.org/async-tiff/latest/)

## Features

- Support for tiled TIFF images.
- Read directly from object storage providers, via the `object_store` crate.
- Support for user-defined decompression algorithms.
- Tile request merging and concurrency.

## Example

```rust
# tokio_test::block_on(async {
use std::sync::Arc;
use std::env::current_dir;

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

It additionally exposes geospatial-specific TIFF tag metadata.

### Tests

Download the following file for use in the tests.

```shell
aws s3 cp s3://naip-visualization/ny/2022/60cm/rgb/40073/m_4007307_sw_18_060_20220803.tif ./ --request-payer
aws s3 cp s3://prd-tnm/StagedProducts/Elevation/13/TIFF/current/s14w171/USGS_13_s14w171.tif ./ --no-sign-request --region us-west-2
```
