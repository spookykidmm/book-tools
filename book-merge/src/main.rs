use anyhow::Result;
use book_core::merger::{Merger, MergeMode};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "book-merge")]
#[command(about = "Merge MP3 chapters into M4B")]
struct Args {
    #[arg(help = "Directory containing MP3 files")]
    input: PathBuf,

    #[arg(short, long, help = "Output directory")]
    output: Option<PathBuf>,

    #[arg(short, long, help = "Output filename (without extension)")]
    name: Option<String>,

    #[arg(short, long, default_value = "copy", help = "Mode: copy or encode")]
    mode: String,

    #[arg(long, help = "Dry run")]
    dry_run: bool,

    #[arg(short, long, help = "Verbose output")]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let output_dir = args.output.unwrap_or_else(|| {
        args.input.parent().unwrap_or(&args.input).join("merged")
    });

    if !output_dir.exists() && !args.dry_run {
        std::fs::create_dir_all(&output_dir)?;
    }

    let name = args.name.unwrap_or_else(|| {
        args.input.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });

    let mode = match args.mode.as_str() {
        "copy" => MergeMode::Copy,
        "encode" => MergeMode::Encode,
        _ => MergeMode::Copy,
    };

    let mut merger = Merger::new();
    merger.mode = mode;

    if args.dry_run {
        println!("[DRY RUN] Would merge MP3s from: {}", args.input.display());
        println!("  Output: {}.m4b", name);
        return Ok(());
    }

    let output = merger.merge(&args.input, &name, &output_dir)?;

    if args.verbose {
        println!("✅ Merged: {}", output.display());
        if let Ok(metadata) = output.metadata() {
            println!("  Size: {:.2} MB", metadata.len() as f64 / (1024.0 * 1024.0));
        }
    }

    Ok(())
}
