# Changelog

## [0.3.0] - 2026-04-01

### What's Changed

* test: Setup Rust benchmarks by @weiji14 in https://github.com/developmentseed/async-tiff/pull/139
* chore: Add additional license copyright from image-tiff by @kylebarron in https://github.com/developmentseed/async-tiff/pull/200
* feat!: Handle transparency/nodata bit masks by @kylebarron in https://github.com/developmentseed/async-tiff/pull/205
* feat: Add webp decompression support by @kylebarron in https://github.com/developmentseed/async-tiff/pull/171
* feat!: Rename `CompressionMethod` to `Compression` by @kylebarron in https://github.com/developmentseed/async-tiff/pull/213
* feat!: Expose colormap as unaltered `[u16]`; implement buffer protocol exchange by @kylebarron in https://github.com/developmentseed/async-tiff/pull/219
* feat: Change `fetch_tiles` to take tuples of `(x, y)` instead of two separate vecs by @kylebarron in https://github.com/developmentseed/async-tiff/pull/229
* feat: LZMA decoder by @kylebarron in https://github.com/developmentseed/async-tiff/pull/230
* feat: support LERC, LERC_DEFLATE, LERC_ZSTD decompression by @kylebarron in https://github.com/developmentseed/async-tiff/pull/204
* feat!: Add support for band-interleaved data by @kylebarron in https://github.com/developmentseed/async-tiff/pull/240
* feat: Define `ExtraSamples` enum by @kylebarron in https://github.com/developmentseed/async-tiff/pull/258
* feat: Expose lerc_parameters as ifd attribute by @kylebarron in https://github.com/developmentseed/async-tiff/pull/262
* feat: Expose API to find byte range of an internal COG tile by @kylebarron in https://github.com/developmentseed/async-tiff/pull/261
* refactor: refactor predictor.rs for simplicity by @kylebarron in https://github.com/developmentseed/async-tiff/pull/265
* chore(deps): update lzma-rust2 requirement from 0.15.7 to 0.16.2 by @dependabot[bot] in https://github.com/developmentseed/async-tiff/pull/286
* ci: Run linter workflows on ubuntu-slim or ubuntu-24.04-arm by @weiji14 in https://github.com/developmentseed/async-tiff/pull/274

### New Contributors

* @dependabot[bot] made their first contribution in https://github.com/developmentseed/async-tiff/pull/281

**Full Changelog**: https://github.com/developmentseed/async-tiff/compare/rust-v0.2.0...rust-v0.3.0

## [0.2.0] - 2026-01-26

### Feature Changes

- feat!: Parse ModelTransformation tag by @kylebarron in https://github.com/developmentseed/async-tiff/pull/179
- chore!: Rename `SampleFormat::IEEEFP` to `SampleFormat::Float` by @kylebarron in https://github.com/developmentseed/async-tiff/pull/184
- Split traits to get image bytes and metadata bytes by @kylebarron in https://github.com/developmentseed/async-tiff/pull/79
- Refactor reading TIFF metadata by @kylebarron in https://github.com/developmentseed/async-tiff/pull/82
- Feature gate `object_store` and `reqwest` by @feefladder in https://github.com/developmentseed/async-tiff/pull/71
- added predictors by @feefladder in https://github.com/developmentseed/async-tiff/pull/86
- feat: Add ZSTD Decoder by @nivdee in https://github.com/developmentseed/async-tiff/pull/157
- feat: Exponential read-ahead cache by @kylebarron in https://github.com/developmentseed/async-tiff/pull/140
- feat: Use `async-trait` instead of boxed futures for simpler trait interface by @kylebarron in https://github.com/developmentseed/async-tiff/pull/147
- feat: Include Endianness as property of TIFF struct by @kylebarron in https://github.com/developmentseed/async-tiff/pull/149
- fix: Skip unknown GeoTag keys by @kylebarron in https://github.com/developmentseed/async-tiff/pull/134
- feat: Exponential read-ahead cache by @kylebarron in https://github.com/developmentseed/async-tiff/pull/140
- feat: Update `Decoder` trait to return `Vec<u8>` instead of `Bytes` by @kylebarron in https://github.com/developmentseed/async-tiff/pull/166
- feat: Rust-side Array concept and `ndarray` integration by @kylebarron in https://github.com/developmentseed/async-tiff/pull/165
- feat: Implement Array helper for structured, zero-copy data sharing with numpy by @kylebarron in https://github.com/developmentseed/async-tiff/pull/164
- feat: add jpeg2k decoder as optional feature by @pmarks in https://github.com/developmentseed/async-tiff/pull/162
- feat: Expose gdal_nodata and gdal_metadata tags by @kylebarron in https://github.com/developmentseed/async-tiff/pull/169
- docs: Add TIFF references to develop.md by @kylebarron in https://github.com/developmentseed/async-tiff/pull/170
- fix: Add handling of unaligned tiles on image border by @kylebarron in https://github.com/developmentseed/async-tiff/pull/180

### Documentation Changes

- docs: Add docs for `TagValue` by @kylebarron in https://github.com/developmentseed/async-tiff/pull/185
- docs: Readme edits by @kylebarron in https://github.com/developmentseed/async-tiff/pull/187
- docs: Validate that Rust docs build without warnings; fix docs links by @kylebarron in https://github.com/developmentseed/async-tiff/pull/189

### Other

- test: Reorganize Rust tests to live inside `src/` by @kylebarron in https://github.com/developmentseed/async-tiff/pull/177
- Test opening single-channel OME-TIFF file by @weiji14 in https://github.com/developmentseed/async-tiff/pull/102

## New Contributors

- @feefladder made their first contribution in https://github.com/developmentseed/async-tiff/pull/71
- @weiji14 made their first contribution in https://github.com/developmentseed/async-tiff/pull/92
- @alukach made their first contribution in https://github.com/developmentseed/async-tiff/pull/138
- @nivdee made their first contribution in https://github.com/developmentseed/async-tiff/pull/157
- @pmarks made their first contribution in https://github.com/developmentseed/async-tiff/pull/162

**Full Changelog**: https://github.com/developmentseed/async-tiff/compare/rust-v0.1.0...rust-v0.2.0

## [0.1.0] - 2025-03-14

- Initial release.
- Includes support for reading metadata out of TIFF files in an async way.
