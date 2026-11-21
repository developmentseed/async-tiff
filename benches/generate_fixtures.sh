#!/usr/bin/env bash
# Generate the GeoTIFF fixtures used by the Rust benchmarks.
#
# Requires the GDAL command line tools (`gdal_translate`). The source image is a
# public Sentinel-2 True Colour Image (10980x10980, 3-band uint8, JPEG-in-COG),
# downloaded once into benches/data/ and then re-encoded into one file per
# codec/predictor combination. All outputs are gitignored (*.tif).
#
# Usage: benches/generate_fixtures.sh [output_dir]
#
# After running, `cargo bench` works from the repository root.
set -euo pipefail

OUT_DIR="${1:-$(cd "$(dirname "$0")" && pwd)/data}"
SOURCE_URL="https://sentinel-cogs.s3.us-west-2.amazonaws.com/sentinel-s2-l2a-cogs/12/S/UF/2022/6/S2B_12SUF_20220609_0_L2A/TCI.tif"
SOURCE="$OUT_DIR/TCI.tif"

mkdir -p "$OUT_DIR"

if [ ! -f "$SOURCE" ]; then
    echo "Downloading source image to $SOURCE"
    curl -fsSL --retry 3 -o "$SOURCE" "$SOURCE_URL"
fi

# Convert the source into a tiled GeoTIFF with the given creation options.
# convert <output name> [gdal_translate options...]
convert() {
    local name="$1"
    shift
    local out="$OUT_DIR/$name"
    if [ -f "$out" ]; then
        echo "Skipping $name (exists)"
        return
    fi
    echo "Generating $name"
    gdal_translate -q -of GTiff -co TILED=YES -co NUM_THREADS=ALL_CPUS "$@" "$SOURCE" "$out"
}

# End-to-end fixture used by benches/read_tiff.rs (default 256px GDAL blocks).
convert TCI_lzw.tif -co COMPRESS=LZW -co PREDICTOR=2

# Per-codec decode fixtures used by benches/decode.rs (512px blocks).
BLOCK=(-co BLOCKXSIZE=512 -co BLOCKYSIZE=512)
convert deflate_u8_pred2.tif "${BLOCK[@]}" -co COMPRESS=DEFLATE -co PREDICTOR=2
convert lzw_u8_pred2.tif "${BLOCK[@]}" -co COMPRESS=LZW -co PREDICTOR=2
convert zstd_u8_pred2.tif "${BLOCK[@]}" -co COMPRESS=ZSTD -co PREDICTOR=2
convert zstd_u8.tif "${BLOCK[@]}" -co COMPRESS=ZSTD
convert jpeg_ycbcr.tif "${BLOCK[@]}" -co COMPRESS=JPEG -co PHOTOMETRIC=YCBCR
convert deflate_f32_pred3.tif "${BLOCK[@]}" -ot Float32 -co COMPRESS=DEFLATE -co PREDICTOR=3

ls -lh "$OUT_DIR"
