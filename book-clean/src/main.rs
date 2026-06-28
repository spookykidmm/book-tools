use anyhow::Result;
use book_core::cleaner::Cleaner;
use book_core::metadata::MetadataExtractor;
use book_core::types::{MetadataSource, Metadata};
use book_core::config::Config;
use clap::Parser;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "book-clean")]
#[command(about = "Clean filenames and extract metadata")]
struct Args {
    #[arg(help = "Manifest JSON file or directory")]
    input: PathBuf,

    #[arg(short = 'o', long, help = "Output manifest with metadata")]
    output: Option<PathBuf>,

    #[arg(short, long, help = "Dry run (don't actually rename)")]
    dry_run: bool,

    #[arg(short, long, help = "Verbose output")]
    verbose: bool,

    #[arg(long, help = "Config file path")]
    config: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Load config
    let config_path = args.config.unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config/book-tools/mappings.toml")
    });

    let config = if config_path.exists() {
        Config::from_file(&config_path)?
    } else {
        eprintln!("⚠️ Config file not found at: {}", config_path.display());
        eprintln!("   Using default mappings (may be less accurate)");
        eprintln!("   Create config at: ~/.config/book-tools/mappings.toml");
        Config::default_config()
    };

    // Check if mappings are loaded
    if config.mappings.authors.is_empty() {
        eprintln!("⚠️ No mappings loaded! Author detection will be limited.");
        eprintln!("   Add mappings to: ~/.config/book-tools/mappings.toml");
    }

    let cleaner = Cleaner::with_mappings(config.mappings);
    let metadata_extractor = MetadataExtractor::new();

    let mut manifest = if args.input.is_dir() {
        let scanner = book_core::scanner::Scanner::new();
        scanner.scan(&args.input)?
    } else {
        let content = fs::read_to_string(&args.input)?;
        serde_json::from_str(&content)?
    };

    for file in &mut manifest.files {
        let embedded_metadata = metadata_extractor.extract_from_file(&file.path)?;
        
        let clean = cleaner.clean_filename(&file.name);
        let clean_clone = clean.clone();
        file.cleaned_name = Some(clean);
        
        let (author, title, narrator, series) = cleaner.extract_metadata(file);

        let mut inferred = Metadata::default();
        if let Some(a) = author {
            inferred.author = Some(a);
        }
        if let Some(t) = title {
            inferred.title = Some(t);
        }
        if let Some(n) = narrator {
            inferred.narrator = Some(n);
        }
        if let Some(s) = series {
            inferred.series = Some(s);
        }

        // Choose best source based on confidence scoring
        let embedded_confidence = embedded_metadata.author.is_some() as u8 + 
                                   embedded_metadata.title.is_some() as u8 +
                                   embedded_metadata.duration.is_some() as u8;
        let inferred_confidence = inferred.author.is_some() as u8 +
                                   inferred.title.is_some() as u8;

        let (chosen_source, chosen_metadata) = if embedded_confidence >= inferred_confidence && embedded_confidence > 0 {
            let mut meta = embedded_metadata.clone();
            // Fill in missing fields from inferred
            if meta.author.is_none() {
                meta.author = inferred.author.clone();
            }
            if meta.title.is_none() {
                meta.title = inferred.title.clone();
            }
            if meta.series.is_none() && inferred.series.is_some() {
                meta.series = inferred.series.clone();
            }
            if meta.narrator.is_none() && inferred.narrator.is_some() {
                meta.narrator = inferred.narrator.clone();
            }
            (MetadataSource::Embedded, meta)
        } else if inferred_confidence > 0 {
            (MetadataSource::Mapping, inferred.clone())
        } else {
            // Fallback to filename
            let mut fallback = Metadata::default();
            fallback.title = Some(file.name.clone());
            (MetadataSource::Filename, fallback)
        };

        file.metadata = Some(chosen_metadata);
        file.inferred = Some(inferred);
        file.chosen_source = chosen_source;

        if args.verbose {
            println!("📁 {}", file.path.display());
            println!("  Original: {}", file.name);
            println!("  Cleaned:  {}", clean_clone);
            println!("  Source:   {:?} (confidence: {:.2})", file.chosen_source, file.chosen_source.confidence());
            if let Some(ref meta) = file.metadata {
                if let Some(a) = &meta.author {
                    println!("  Author:   {}", a);
                }
                if let Some(n) = &meta.narrator {
                    println!("  Narrator: {}", n);
                }
                if let Some(t) = &meta.title {
                    println!("  Title:    {}", t);
                }
                if let Some(s) = &meta.series {
                    println!("  Series:   {}", s);
                }
                if let Some(y) = meta.year {
                    println!("  Year:     {}", y);
                }
                if let Some(b) = meta.bitrate {
                    println!("  Bitrate:  {} kbps", b);
                }
                if let Some(d) = meta.duration {
                    println!("  Duration: {}s", d);
                }
            }
            println!();
        }
    }

    let output_path = if let Some(out) = args.output {
        out
    } else {
        let default_parent = PathBuf::from(".");
        let parent = args.input.parent().unwrap_or(&default_parent);
        parent.join("manifest_with_metadata.json")
    };

    let json = serde_json::to_string_pretty(&manifest)?;
    fs::write(&output_path, json)?;

    if args.verbose {
        println!("✅ Saved manifest with metadata to: {}", output_path.display());
    }

    Ok(())
}
