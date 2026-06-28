use anyhow::Result;
use book_core::organizer::Organizer;
use book_core::hashing::hash_file;
use book_core::types::{Metadata, FileStatus};
use clap::Parser;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "book-organize")]
#[command(about = "Organize files into a clean structure")]
struct Args {
    #[arg(help = "Manifest JSON file (with metadata)")]
    manifest: PathBuf,

    #[arg(short, long, help = "Output directory")]
    output: PathBuf,

    #[arg(long, help = "Dry run")]
    dry_run: bool,

    #[arg(short, long, help = "Verbose output")]
    verbose: bool,

    #[arg(long, help = "Skip hash dedupe (faster)")]
    no_hash: bool,

    #[arg(long, help = "Skip verification (faster)")]
    no_verify: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let content = fs::read_to_string(&args.manifest)?;
    let mut manifest: book_core::types::Manifest = serde_json::from_str(&content)?;

    // 1. Hash-based dedupe (if not disabled)
    if !args.no_hash {
        println!("🔍 Computing file hashes for dedupe...");
        let mut hashes: HashMap<String, Vec<PathBuf>> = HashMap::new();
        
        for file in &manifest.files {
            if file.file_type == book_core::types::FileType::Audio ||
               file.file_type == book_core::types::FileType::Ebook {
                if let Ok(hash) = hash_file(&file.path) {
                    hashes.entry(hash).or_insert_with(Vec::new).push(file.path.clone());
                }
            }
        }

        let duplicates: Vec<_> = hashes.iter()
            .filter(|(_, paths)| paths.len() > 1)
            .collect();

        if !duplicates.is_empty() {
            println!("📊 Found {} duplicate groups", duplicates.len());
            for (hash, paths) in duplicates {
                if args.verbose {
                    println!("  Duplicate group: {}", hash);
                    for path in paths {
                        println!("    - {}", path.display());
                    }
                }
                for path in &paths[1..] {
                    println!("  ⚠️ Duplicate: {}", path.display());
                }
            }
        } else {
            println!("✅ No duplicates found");
        }
        println!();
    }

    // 2. Organize with verification
    let output_path = args.output.clone();
    let mut organizer = Organizer::new(output_path);
    organizer.move_files = false;
    organizer.dry_run = args.dry_run;
    organizer.verify = !args.no_verify;

    if args.dry_run {
        println!("[DRY RUN] Would organize files into: {}", organizer.output_dir.display());
        for file in &manifest.files {
            let dest = organizer.destination_path(file)?;
            let default_meta = Metadata::default();
            let meta = file.metadata.as_ref().unwrap_or(&default_meta);
            let author = meta.author.as_deref().unwrap_or("Unknown Author");
            println!("  {} → {}/{}", file.path.display(), author, dest.file_name().unwrap_or_default().to_string_lossy());
        }
        return Ok(());
    }

    // Organize and verify
    let organized = organizer.organize(&mut manifest)?;
    let verified_count = manifest.files.iter().filter(|f| f.status == FileStatus::Verified).count();
    println!("✅ Organized {} files ({} verified)", organized.len(), verified_count);

    // Save updated manifest with statuses
    let status_output = args.output.join("manifest_status.json");
    let json = serde_json::to_string_pretty(&manifest)?;
    fs::write(&status_output, json)?;
    println!("✅ Saved status manifest to: {}", status_output.display());

    if args.verbose {
        for path in organized {
            println!("  {}", path.display());
        }
    }

    Ok(())
}
