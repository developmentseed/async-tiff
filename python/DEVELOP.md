```
uv sync --no-install-package async-tiff
uv run --no-project maturin develop --uv
uv run --no-project mkdocs serve
uv run --no-project pytest tests --verbose
```

## TIFF References:

- [A Handy Introduction to Cloud Optimized GeoTIFFs](https://medium.com/planet-stories/a-handy-introduction-to-cloud-optimized-geotiffs-1f2c9e716ec3)
- [TIFF: IFD and SubIFD](https://dpb587.me/entries/tiff-ifd-and-subifd-20240226)
- [TIFF File structure diagram](https://github.com/cogeotiff/cog-spec/issues/6)
