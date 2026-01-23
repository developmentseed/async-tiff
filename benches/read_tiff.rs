//! Benchmarks on opening and decoding a GeoTIFF
//!
//! Steps:
//! 1. Download Sentinel-2 True-Colour Image (TCI) file from
//!    https://sentinel-cogs.s3.us-west-2.amazonaws.com/sentinel-s2-l2a-cogs/12/S/UF/2022/6/S2B_12SUF_20220609_0_L2A/TCI.tif
//!    to `benches/` folder, applying LZW compression with Horizontal differencing
//!    predictor using the following command:
//!    `gdal raster convert --co COMPRESS=LZW --co TILED=YES --co PREDICTOR=2 benches/TCI.tif benches/TCI_lzw.tif`
//! 2. Run `cargo bench`

use std::path::PathBuf;
use std::sync::Arc;

use async_tiff::decoder::DecoderRegistry;
use async_tiff::error::{AsyncTiffError, AsyncTiffResult};
use async_tiff::metadata::cache::ReadaheadMetadataCache;
use async_tiff::metadata::TiffMetadataReader;
use async_tiff::reader::{AsyncFileReader, ObjectReader};
use async_tiff::{Array, ImageFileDirectory, Tile};
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use object_store::path::Path;
use object_store::{parse_url, ObjectStore};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rayon::ThreadPoolBuilder;
use reqwest::Url;
use tokio::runtime;

// Retrieve all TIFF tiles (with their compressed bytes) from the first IFD
async fn read_tiles<R: AsyncFileReader + Clone>(reader: R) -> AsyncTiffResult<Vec<Tile>> {
    // Read metadata header
    let cached_reader = ReadaheadMetadataCache::new(reader.clone());
    let mut metadata_reader = TiffMetadataReader::try_open(&cached_reader).await?;

    // Read Image File Directories
    let ifds: Vec<ImageFileDirectory> = metadata_reader.read_all_ifds(&cached_reader).await?;

    assert_eq!(ifds.len(), 1); // should have only 1 IFD
    let ifd: &ImageFileDirectory = ifds.first().ok_or(AsyncTiffError::General(
        "unable to read first IFD".to_string(),
    ))?;

    let (x_count, y_count): (usize, usize) = ifd.tile_count().ok_or(AsyncTiffError::General(
        "unable to get IFD count".to_string(),
    ))?;

    // Get cartesian product of x and y tile ids
    let x_ids: Vec<usize> = (0..x_count)
        .flat_map(|i| (0..y_count).map(move |_j| i))
        .collect();
    let y_ids: Vec<usize> = (0..x_count).flat_map(|_i| 0..y_count).collect();

    let tiles: Vec<Tile> = ifd.fetch_tiles(&x_ids, &y_ids, &reader).await?;
    Ok(tiles)
}

// Open TIFF file, fetching compressed tile bytes in async manner
fn open_tiff(fpath: &str) -> AsyncTiffResult<Vec<Tile>> {
    let abs_path: PathBuf = std::path::Path::new(fpath).canonicalize()?;
    let tif_url: Url = Url::from_file_path(abs_path).expect("Failed to parse url: {abs_path}");
    let (store, path): (Box<dyn ObjectStore>, Path) = parse_url(&tif_url)?;

    let reader = ObjectReader::new(Arc::new(store), path);

    // Initialize async runtime
    let runtime = runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    // Get list of tiles in TIFF file stream (using tokio async runtime)
    let tiles: Vec<Tile> = runtime.block_on(read_tiles(reader)).unwrap();
    assert_eq!(tiles.len(), 1849); // x_count:43 * y_count:43 = 1849

    Ok(tiles)
}

// Do actual decoding of compressed TIFF tiles using multi-threading
fn decode_tiff(tiles: Vec<Tile>) -> AsyncTiffResult<Vec<Array>> {
    let decoder_registry = DecoderRegistry::default();

    // Do actual decoding of TIFF tile data (multi-threaded using rayon)
    let pool = ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .map_err(|err| AsyncTiffError::External(Box::new(err)))?;

    let tile_arrays: Vec<Array> = pool.install(|| {
        tiles
            .into_par_iter()
            .map(|tile| tile.decode(&decoder_registry).unwrap())
            .collect()
    });
    assert_eq!(tile_arrays.len(), 363528192); // 363528192 = 1849 * 196608, should be
                                              // 361681200 if mask padding is removed

    Ok(tile_arrays)
}

fn read_tiff(fpath: &str) -> AsyncTiffResult<()> {
    let compressed_tiles: Vec<Tile> = open_tiff(fpath)?;
    let _decoded_tiles: Vec<Array> = decode_tiff(compressed_tiles)?;
    Ok(())
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_tiff");

    let fsize: u64 = std::fs::metadata("benches/TCI_lzw.tif").unwrap().len();
    group.throughput(Throughput::BytesDecimal(fsize)); // 55MB filesize

    // CPU decoding using async-tiff
    group.sample_size(30);
    group.bench_function("async-tiff", move |b| {
        b.iter(|| read_tiff("benches/TCI_lzw.tif"))
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
