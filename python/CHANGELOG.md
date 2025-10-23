# Changelog

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
- chore: Bump _obstore submodule for latest store creation types #125

### New Contributors

- @feefladder made their first contribution in https://github.com/developmentseed/async-tiff/pull/71
- @weiji14 made their first contribution in https://github.com/developmentseed/async-tiff/pull/92

**Full Changelog**: https://github.com/developmentseed/async-tiff/compare/py-v0.1.0...py-v0.1.1

## [0.1.0] - 2025-03-18

- Initial release.
