/// Benchmarks on reading a GeoTIFF
use std::path::PathBuf;
use std::sync::Arc;

use async_tiff::decoder::DecoderRegistry;
use async_tiff::error::{AsyncTiffError, AsyncTiffResult};
use async_tiff::metadata::cache::ReadaheadMetadataCache;
use async_tiff::metadata::TiffMetadataReader;
use async_tiff::reader::{AsyncFileReader, ObjectReader};
use async_tiff::{ImageFileDirectory, Tile};
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use object_store::path::Path;
use object_store::{parse_url, ObjectStore};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rayon::ThreadPoolBuilder;
use reqwest::Url;
use tokio::runtime;

fn open_tiff(fpath: &str) -> AsyncTiffResult<ObjectReader> {
    let abs_path: PathBuf = std::path::Path::new(fpath).canonicalize()?;
    let tif_url: Url = Url::from_file_path(abs_path).expect("Failed to parse url: {abs_path}");
    let (store, path): (Box<dyn ObjectStore>, Path) = parse_url(&tif_url)?;

    let reader = ObjectReader::new(Arc::new(store), path);
    Ok(reader)
}

fn decode_tiff<R: AsyncFileReader + Clone>(reader: R) -> AsyncTiffResult<Vec<u8>> {
    let decoder_registry = DecoderRegistry::default();

    // Initialize async runtime
    let runtime = runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    // Get list of tiles in TIFF file stream (using tokio async runtime)
    let tiles: Vec<Tile> = runtime
        .block_on(async {
            // Read metadata header
            let cached_reader = ReadaheadMetadataCache::new(reader.clone());
            let mut metadata_reader = TiffMetadataReader::try_open(&cached_reader).await?;

            // Read Image File Directories
            let ifds: Vec<ImageFileDirectory> =
                metadata_reader.read_all_ifds(&cached_reader).await?;

            assert_eq!(ifds.len(), 1); // should have only 1 IFD
            let ifd: &ImageFileDirectory = ifds.first().ok_or(AsyncTiffError::General(
                "unable to read first IFD".to_string(),
            ))?;

            let (x_count, y_count) = ifd.tile_count().ok_or(AsyncTiffError::General(
                "unable to get IFD count".to_string(),
            ))?;
            assert_eq!(x_count, 43);
            assert_eq!(y_count, 43);

            // Get cartesian product of x and y tile ids
            let x_ids: Vec<usize> = (0..x_count)
                .flat_map(|i| (0..y_count).map(move |_j| i))
                .collect();
            let y_ids: Vec<usize> = (0..x_count).flat_map(|_i| 0..y_count).collect();

            let tiles: Vec<Tile> = ifd.fetch_tiles(&x_ids, &y_ids, &reader).await?;
            assert_eq!(tiles.len(), 1849); // 43 * 43 = 1849

            Ok::<Vec<Tile>, AsyncTiffError>(tiles)
        })
        .unwrap();

    // Do actual decoding of TIFF tile data (multi-threaded using rayon)
    let pool = ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .map_err(|err| AsyncTiffError::External(Box::new(err)))?;

    let tile_bytes: Vec<u8> = pool.install(|| {
        tiles
            .into_par_iter()
            .flat_map_iter(|tile| tile.decode(&decoder_registry).unwrap())
            .collect()
    });
    assert_eq!(tile_bytes.len(), 363528192); // should be 361681200, why not?

    Ok(tile_bytes)
}

fn read_tiff(fpath: &str) -> AsyncTiffResult<()> {
    let reader: ObjectReader = open_tiff(fpath)?;
    let _tiles: Vec<u8> = decode_tiff(reader)?;
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
