# async-tiff

An async [TIFF](https://en.wikipedia.org/wiki/TIFF) reader.

The existing [`tiff` crate](https://crates.io/crates/tiff) is great, but only supports synchronous reading of TIFF files. Furthermore, due to low maintenance bandwidth it is not designed for extensibility (see [#250](https://github.com/image-rs/image-tiff/issues/250)).

This crate is designed to be a minimal, low-level interface to read tiled TIFF files in an async way.

It additionally exposes geospatial-specific TIFF tag metadata.

### Tests

Download the following file for use in the tests.

```shell
aws s3 cp s3://naip-visualization/ny/2022/60cm/rgb/40073/m_4007307_sw_18_060_20220803.tif ./ --request-payer
```
