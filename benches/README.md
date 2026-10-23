# Benchmarks

Criterion benchmarks for the Rust crate. Local runs on one machine, back to back against a saved
baseline, are the numbers we trust for a PR. CI runs the same benchmarks under CodSpeed in
simulation mode, which is deterministic but counts instructions rather than time, so treat it as
a regression tripwire rather than a measurement.

## Fixtures

Fixtures are generated from a public Sentinel-2 True Colour Image (10980x10980, 3-band uint8)
and are gitignored. Generating them requires the GDAL command line tools:

```sh
benches/generate_fixtures.sh
```

This downloads the source image once into `benches/data/` and writes one file per
codec/predictor combination (deflate, LZW, zstd with and without predictor, JPEG YCbCr, and
float32 deflate with the floating-point predictor), plus the LZW file used by the end-to-end
benchmark.

## Benchmarks

- `decode` (`benches/decode.rs`): decodes a fixed 4x4 block of interior 512px tiles from each
  fixture. The timed region is codec decompression, predictor reversal and typed-array
  construction only; no I/O. One result per fixture, with decoded-bytes throughput.
- `read_tiff` (`benches/read_tiff.rs`): end to end. Opens the LZW file from the local
  filesystem, fetches all 1849 tiles, and decodes them on a 4-thread rayon pool.

## Comparing a change

On `main`, save a baseline:

```sh
cargo bench --bench decode -- --save-baseline main
```

On your branch, compare against it:

```sh
cargo bench --bench decode -- --baseline main
```

Criterion prints the per-benchmark change with a confidence interval and whether it is
statistically significant. For a compact table, install [`critcmp`](https://github.com/BurntSushi/critcmp):

```sh
cargo bench --bench decode -- --save-baseline pr
critcmp main pr
```

Paste that table into the PR description. Close other heavy processes while measuring and keep
the laptop plugged in; a few percent of noise between runs is normal, so look for changes that
criterion reports as significant and that are larger than run-to-run variation on `main`.
