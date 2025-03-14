# async-tiff

An async, low-level [TIFF](https://en.wikipedia.org/wiki/TIFF) reader.

## Features

- Support for tiled TIFF images.
- Read directly from object storage providers, via the `object_store` crate.
- Support for user-defined decompression algorithms.
- Tile request merging and concurrency.

[Full documentation](https://docs.rs/async-tiff/).

## Background

The existing [`tiff` crate](https://crates.io/crates/tiff) is great, but only supports synchronous reading of TIFF files. Furthermore, due to low maintenance bandwidth it is not designed for extensibility (see [#250](https://github.com/image-rs/image-tiff/issues/250)).

It additionally exposes geospatial-specific TIFF tag metadata.

### Tests

Download the following file for use in the tests.

```shell
aws s3 cp s3://naip-visualization/ny/2022/60cm/rgb/40073/m_4007307_sw_18_060_20220803.tif ./ --request-payer
```
