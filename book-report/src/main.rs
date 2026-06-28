use anyhow::Result;
use clap::Parser;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "book-report")]
#[command(about = "Generate a library report")]
struct Args {
    #[arg(help = "Manifest JSON file or directory")]
    input: PathBuf,

    #[arg(short, long, help = "Output report file")]
    output: Option<PathBuf>,

    #[arg(short, long, help = "Format: html, txt, json")]
    format: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let manifest = if args.input.is_dir() {
        let scanner = book_core::scanner::Scanner::new();
        scanner.scan(&args.input)?
    } else {
        let content = fs::read_to_string(&args.input)?;
        serde_json::from_str(&content)?
    };

    let format = args.format.as_deref().unwrap_or("txt");
    let report = match format {
        "txt" => generate_text_report(&manifest),
        "json" => generate_json_report(&manifest),
        _ => generate_text_report(&manifest),
    }?;

    if let Some(output) = args.output {
        fs::write(output, report)?;
    } else {
        println!("{}", report);
    }

    Ok(())
}

fn generate_text_report(manifest: &book_core::types::Manifest) -> Result<String> {
    let mut report = String::new();
    report.push_str("📚 LIBRARY REPORT\n");
    report.push_str("================\n\n");
    report.push_str(&format!("Source: {}\n", manifest.source.display()));
    report.push_str(&format!("Generated: {}\n\n", manifest.generated));
    report.push_str("📊 STATISTICS\n");
    report.push_str("-------------\n");
    report.push_str(&format!("Total files: {}\n", manifest.stats.total_files));
    report.push_str(&format!("Total size: {:.2} MB\n", manifest.stats.total_size as f64 / (1024.0 * 1024.0)));
    report.push_str(&format!("  🎵 Audio: {} files\n", manifest.stats.audio_files));
    report.push_str(&format!("  📖 Ebooks: {} files\n", manifest.stats.ebook_files));
    report.push_str(&format!("  🎨 Comics: {} files\n", manifest.stats.comic_files));
    report.push_str(&format!("  🗑️  Junk: {} files\n", manifest.stats.junk_files));
    Ok(report)
}

fn generate_json_report(manifest: &book_core::types::Manifest) -> Result<String> {
    Ok(serde_json::to_string_pretty(manifest)?)
}
