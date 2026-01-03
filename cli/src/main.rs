mod commands;
mod storage;

use std::sync::Arc;

use anyhow::Result;
use async_tiff::metadata::cache::ReadaheadMetadataCache;
use async_tiff::metadata::TiffMetadataReader;
use async_tiff::reader::{AsyncFileReader, ObjectReader};
use async_tiff::TIFF;
use clap::Parser;

#[derive(Parser)]
#[command(name = "atiff")]
#[command(about = "Command-line tools for async-tiff", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Display information about TIFF files
    Info {
        /// Path to TIFF file (local or s3://)
        path: String,

        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: OutputFormat,

        /// Compute and display statistics
        #[arg(long)]
        stats: bool,

        /// Only show specific metadata
        #[arg(long)]
        count: bool,

        #[arg(long)]
        dtype: bool,

        #[arg(long)]
        shape: bool,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum OutputFormat {
    Text,
    Json,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Info {
            path,
            format,
            stats: _,
            count: _,
            dtype: _,
            shape: _,
        } => {
            let (store, object_path) = storage::create_object_store(&path)?;

            // Create reader and cache
            let reader = Arc::new(ObjectReader::new(store, object_path)) as Arc<dyn AsyncFileReader>;
            let cached_reader = ReadaheadMetadataCache::new(reader.clone());

            // Read metadata and IFDs
            let mut metadata_reader = TiffMetadataReader::try_open(&cached_reader).await?;
            let ifds = metadata_reader.read_all_ifds(&cached_reader).await?;
            let tiff = TIFF::new(ifds, metadata_reader.endianness());

            let format = match format {
                OutputFormat::Text => commands::info::OutputFormat::Text,
                OutputFormat::Json => commands::info::OutputFormat::Json,
            };

            commands::info::execute(&tiff, format).await?;
        }
    }

    Ok(())
}
