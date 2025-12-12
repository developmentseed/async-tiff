# Changelog

## [0.3.0] - 2025-12-12

### What's Changed

- feat: Exponential read-ahead cache by @kylebarron in https://github.com/developmentseed/async-tiff/pull/140
- feat(python): implement Mapping protocol for IFD and GeoKeyDirectory by @kylebarron in https://github.com/developmentseed/async-tiff/pull/148
- feat: Include Endianness as property of TIFF struct by @kylebarron in https://github.com/developmentseed/async-tiff/pull/149
- fix: Handle non utf-8 characters in OME-XML by @weiji14 in https://github.com/developmentseed/async-tiff/pull/141
- feat: Add ZSTD Decoder by @nivdee in https://github.com/developmentseed/async-tiff/pull/157
- refactor: Use `pyclass(get_all)` for cleaner code by @kylebarron in https://github.com/developmentseed/async-tiff/pull/158
- fix: Skip unknown GeoTag keys by @kylebarron in https://github.com/developmentseed/async-tiff/pull/134
- ci: Deprecate Python 3.9, add testing on Python 3.13 by @kylebarron in https://github.com/developmentseed/async-tiff/pull/129

### New Contributors

- @alukach made their first contribution in https://github.com/developmentseed/async-tiff/pull/138
- @nivdee made their first contribution in https://github.com/developmentseed/async-tiff/pull/157

**Full Changelog**: https://github.com/developmentseed/async-tiff/compare/py-v0.2.0...py-v0.3.0

## [0.2.0] - 2025-10-23

### What's Changed

- Enable pytest-asyncio tests in CI by @weiji14 in https://github.com/developmentseed/async-tiff/pull/92
- Raise FileNotFoundError instead of panic when opening missing files by @weiji14 in https://github.com/developmentseed/async-tiff/pull/93
- Raise TypeError instead of panic on doing fetch_tile from striped TIFFs by @weiji14 in https://github.com/developmentseed/async-tiff/pull/99
- Test opening single-channel OME-TIFF file by @weiji14 in https://github.com/developmentseed/async-tiff/pull/102
- Remove broken symlink when building windows wheels by @maxrjones in https://github.com/developmentseed/async-tiff/pull/120
- chore!: Bump minimum Python version to 3.10 by @kylebarron in https://github.com/developmentseed/async-tiff/pull/122
- chore: Bump pyo3 to 0.26 by @kylebarron in https://github.com/developmentseed/async-tiff/pull/121
- ci: Build abi3 wheels where possible by @kylebarron in https://github.com/developmentseed/async-tiff/pull/123
- chore: Bump \_obstore submodule for latest store creation types #125

### New Contributors

- @feefladder made their first contribution in https://github.com/developmentseed/async-tiff/pull/71
- @weiji14 made their first contribution in https://github.com/developmentseed/async-tiff/pull/92

**Full Changelog**: https://github.com/developmentseed/async-tiff/compare/py-v0.1.0...py-v0.1.1

## [0.1.0] - 2025-03-18

- Initial release.
