use anyhow::Result;
use book_core::metadata::MetadataExtractor;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "book-meta")]
#[command(about = "Extract and normalize metadata")]
struct Args {
    #[arg(help = "File or directory to process")]
    input: PathBuf,

    #[arg(short, long, help = "Output metadata JSON")]
    output: Option<PathBuf>,

    #[arg(short, long, help = "Verbose output")]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let extractor = MetadataExtractor;

    if args.input.is_dir() {
        let scanner = book_core::scanner::Scanner::new();
        let manifest = scanner.scan(&args.input)?;

        for file in &manifest.files {
            let mut metadata = extractor.extract_from_file(&file.path)?;
            let _ = extractor.normalize(&mut metadata);

            if args.verbose {
                println!("📁 {}", file.path.display());
                println!("  Title: {:?}", metadata.title);
                println!("  Author: {:?}", metadata.author);
                println!("  Series: {:?}", metadata.series);
                println!("  Year: {:?}", metadata.year);
                if let Some(d) = metadata.duration {
                    println!("  Duration: {}s", d);
                }
                if let Some(b) = metadata.bitrate {
                    println!("  Bitrate: {}kbps", b);
                }
                println!();
            }
        }
    } else {
        let mut metadata = extractor.extract_from_file(&args.input)?;
        let _ = extractor.normalize(&mut metadata);

        if let Some(output) = args.output {
            let json = serde_json::to_string_pretty(&metadata)?;
            std::fs::write(output, json)?;
        } else if args.verbose {
            println!("📁 {}", args.input.display());
            println!("  Title: {:?}", metadata.title);
            println!("  Author: {:?}", metadata.author);
            println!("  Series: {:?}", metadata.series);
            println!("  Year: {:?}", metadata.year);
        } else {
            println!("{}", serde_json::to_string_pretty(&metadata)?);
        }
    }

    Ok(())
}
