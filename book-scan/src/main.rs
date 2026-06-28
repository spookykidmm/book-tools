use anyhow::Result;
use book_core::scanner::Scanner;
use clap::Parser;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "book-scan")]
#[command(about = "Scan a directory for book files")]
struct Args {
    #[arg(help = "Directory to scan")]
    path: PathBuf,

    #[arg(short, long, help = "Output JSON manifest")]
    output: Option<PathBuf>,

    #[arg(short, long, help = "Verbose output")]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let scanner = Scanner::new();
    let manifest = scanner.scan(&args.path)?;

    if args.verbose {
        println!("📊 Scan Summary");
        println!("===============");
        println!("Source: {}", manifest.source.display());
        println!("Total files: {}", manifest.stats.total_files);
        println!("  🎵 Audio: {}", manifest.stats.audio_files);
        println!("  📖 Ebooks: {}", manifest.stats.ebook_files);
        println!("  🎨 Comics: {}", manifest.stats.comic_files);
        println!("  🗑️  Junk: {}", manifest.stats.junk_files);
        println!("  ❓ Unknown: {}", manifest.stats.unknown_files);
        println!("Total size: {:.2} MB", manifest.stats.total_size as f64 / (1024.0 * 1024.0));
    }

    if let Some(output) = args.output {
        let json = serde_json::to_string_pretty(&manifest)?;
        fs::write(output, json)?;
    } else {
        println!("{}", serde_json::to_string_pretty(&manifest)?);
    }

    Ok(())
}
