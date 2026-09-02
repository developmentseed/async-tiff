use std::ops::Range;
use std::panic::AssertUnwindSafe;

use async_tiff::error::AsyncTiffResult;
use async_tiff::metadata::{MetadataFetch, TiffMetadataReader};
use async_trait::async_trait;
use bytes::Bytes;
use futures::FutureExt;

// In-memory fetch over a byte buffer.
#[derive(Debug)]
struct MemFetch(Bytes);

#[async_trait]
impl MetadataFetch for MemFetch {
    async fn fetch(&self, range: Range<u64>) -> AsyncTiffResult<Bytes> {
        let start = range.start as usize;
        let end = (range.end as usize).min(self.0.len());
        let out = if start >= self.0.len() || start > end {
            Bytes::new()
        } else {
            self.0.slice(start..end)
        };
        Ok(out)
    }
}

async fn parse(data: Bytes) {
    let fetch = MemFetch(data);
    if let Ok(mut r) = TiffMetadataReader::try_open(&fetch).await {
        let _ = r.read(&fetch).await;
    }
}

#[tokio::test]
async fn mutated_tiffs_never_panic() {
    let paths = [
        "fixtures/image-tiff/gradient-1c-32b.tiff",
        "fixtures/image-tiff/palette-1c-1b.tiff",
        "fixtures/image-tiff/quad-lzw-compat.tiff",
        "fixtures/image-tiff/geo-5b.tif",
        "fixtures/other/geogtowgs_subset_USGS_13_s14w171.tif",
    ];
    let mut seeds: Vec<Vec<u8>> = Vec::new();
    for p in paths {
        if let Ok(b) = std::fs::read(p) {
            seeds.push(b);
        }
    }
    assert!(!seeds.is_empty(), "no fixtures found");

    let mut rng: u64 = 0x9e3779b97f4a7c15;
    let mut next = || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };

    for iter in 0..50_000u64 {
        let seed = &seeds[(iter as usize) % seeds.len()];
        let mut b = seed.clone();
        let ops = next() % 6 + 1;
        for _ in 0..ops {
            if b.is_empty() {
                break;
            }
            match next() % 4 {
                0 => {
                    let i = (next() as usize) % b.len();
                    b[i] = (next() & 0xff) as u8;
                }
                1 => {
                    let i = (next() as usize) % b.len();
                    b[i] ^= 1 << (next() % 8);
                }
                2 => b.truncate((next() as usize) % b.len().max(1)),
                _ => {
                    // bump a byte in the IFD area (after the 8-byte header)
                    if b.len() > 12 {
                        let i = 8 + (next() as usize) % (b.len() - 8);
                        b[i] = (next() & 0xff) as u8;
                    }
                }
            }
        }
        let data = Bytes::from(b.clone());
        let res = AssertUnwindSafe(parse(data)).catch_unwind().await;
        if res.is_err() {
            panic!(
                "PANIC at iter {iter} on {} bytes: head={:02x?}",
                b.len(),
                &b[..b.len().min(32)]
            );
        }
    }
}
