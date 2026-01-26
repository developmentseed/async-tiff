# Changelog

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
