`geogtowgs_subset_USGS_13_s14w171.tif` was created from "s3://prd-tnm/StagedProducts/Elevation/13/TIFF/current/s14w171/USGS_13_s14w171.tif" using these commands:

```bash
gdal_translate USGS_13_s14w171.tif tiny.tif -srcwin 0 0 1 1 -co COMPRESS=DEFLATE
listgeo USGS_13_s14w171.tif > metadata.txt  # Then modify to remove information related to spatial extent
cp tiny.tif geogtowgs_subset_USGS_13_s14w171.tif
geotifcp -g metadata.txt tiny.tif geogtowgs_subset_USGS_13_s14w171.tif
listgeo geogtowgs_subset_USGS_13_s14w171.tif
```

and this workspace definition:

```toml
[project]
name = "gdal-workspace"
version = "0.1.0"
description = "workspace for using gdal via pixi"
readme = "README.md"
requires-python = ">=3.12"
dependencies = []

[tool.pixi.workspace]
channels = ["conda-forge"]
platforms = ["osx-arm64"]

[tool.pixi.pypi-dependencies]
gdal-workspace = { path = ".", editable = true }

[tool.pixi.tasks]

[tool.pixi.dependencies]
gdal = ">=3.11.5,<4"
libgdal = ">=3.11.5,<4"
geotiff = ">=1.7.4,<2"
```