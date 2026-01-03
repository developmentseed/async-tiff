use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use object_store::aws::AmazonS3Builder;
use object_store::local::LocalFileSystem;
use object_store::ObjectStore;

/// Create an ObjectStore and path from a file path string.
/// Supports both local file paths and S3 URLs (s3://bucket/key).
pub fn create_object_store(
    path: &str,
) -> Result<(Arc<dyn ObjectStore>, object_store::path::Path)> {
    if let Some(s3_path) = path.strip_prefix("s3://") {
        create_s3_store(s3_path)
    } else {
        create_local_store(path)
    }
}

fn create_s3_store(s3_path: &str) -> Result<(Arc<dyn ObjectStore>, object_store::path::Path)> {
    let (bucket, key) = s3_path
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("Invalid S3 path. Expected format: s3://bucket/path/to/file.tif"))?;

    let store = AmazonS3Builder::from_env()
        .with_bucket_name(bucket)
        .build()?;

    let path = object_store::path::Path::parse(key)?;
    Ok((Arc::new(store), path))
}

fn create_local_store(file_path: &str) -> Result<(Arc<dyn ObjectStore>, object_store::path::Path)> {
    let path_buf = PathBuf::from(file_path);
    let folder = path_buf
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let filename = path_buf
        .file_name()
        .and_then(|f| f.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;

    let store = LocalFileSystem::new_with_prefix(folder)?;
    let path = object_store::path::Path::parse(filename)?;
    Ok((Arc::new(store), path))
}
