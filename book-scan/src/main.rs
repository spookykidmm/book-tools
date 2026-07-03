use anyhow::Result;
use book_core::{scanner::Scanner, registry::Registry, author_db::AuthorDB};
use clap::Parser;
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

    #[arg(long, help = "Update registry with scanned files")]
    registry: Option<String>,

    #[arg(long, help = "Author database path")]
    author_db: Option<String>,
}

// Helper function to extract author from path
fn extract_author_from_path(path: &PathBuf) -> Option<String> {
    // Try to extract author from the path
    // e.g., "Cormac McCarthy - Border Trilogy/" -> "Cormac McCarthy"
    if let Some(parent) = path.parent() {
        if let Some(dir_name) = parent.file_name() {
            let dir_str = dir_name.to_string_lossy();
            let parts: Vec<&str> = dir_str.split(" - ").collect();
            if !parts.is_empty() {
                let first = parts[0].trim();
                // Check if it looks like an author name
                if !first.chars().all(|c| c.is_ascii_digit()) && first.len() > 3 {
                    return Some(first.to_string());
                }
            }
        }
    }
    // Try grandparent
    if let Some(grandparent) = path.parent().and_then(|p| p.parent()) {
        if let Some(dir_name) = grandparent.file_name() {
            let dir_str = dir_name.to_string_lossy();
            let parts: Vec<&str> = dir_str.split(" - ").collect();
            if !parts.is_empty() {
                let first = parts[0].trim();
                if !first.chars().all(|c| c.is_ascii_digit()) && first.len() > 3 {
                    return Some(first.to_string());
                }
            }
        }
    }
    None
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

    // Update registry if requested
    if let Some(db_path) = args.registry {
        let registry = Registry::new(&db_path)?;

        // Initialize author DB if provided
        let author_db = if let Some(ref author_db_path) = args.author_db {
            Some(AuthorDB::new(author_db_path)?)
        } else {
            None
        };

        if author_db.is_some() && args.verbose {
            println!("📖 Author DB loaded: {}", args.author_db.as_ref().unwrap());
        }

        let mut new_count = 0;
        let mut author_count = 0;

        for file in &manifest.files {
            if let Some(hash) = &file.content_hash {
                // Skip if already in registry
                if registry.exists(hash)? {
                    continue;
                }

                // Get title and author from inferred metadata
                let (title, author) = if let Some(inferred) = &file.inferred {
                    (inferred.title.clone(), inferred.author.clone())
                } else {
                    (None, None)
                };

                // Normalize author if author DB is available
                let final_author = if let Some(db) = &author_db {
                    if let Some(ref author_name) = author {
                        // CRITICAL: This is where the author gets added to the DB
                        match db.get_or_create_author(author_name) {
                            Ok(author_record) => {
                                author_count += 1;
                                if args.verbose {
                                    println!("📖 Added author: {} (from '{}')", author_record.canonical_name, author_name);
                                }
                                Some(author_record.canonical_name)
                            }
                            Err(e) => {
                                eprintln!("⚠️ Failed to normalize author '{}': {}", author_name, e);
                                author.clone()
                            }
                        }
                    } else {
                        // No author found, try to infer from path
                        if let Some(path_author) = extract_author_from_path(&file.path) {
                            match db.get_or_create_author(&path_author) {
                                Ok(author_record) => {
                                    author_count += 1;
                                    if args.verbose {
                                        println!("📖 Added author from path: {}", author_record.canonical_name);
                                    }
                                    Some(author_record.canonical_name)
                                }
                                Err(e) => {
                                    eprintln!("⚠️ Failed to normalize author from path '{}': {}", path_author, e);
                                    None
                                }
                            }
                        } else {
                            None
                        }
                    }
                } else {
                    author.clone()
                };

                // Determine file type as string
                let file_type_str = match file.file_type {
                    book_core::types::FileType::Audio => "audio",
                    book_core::types::FileType::Ebook => "ebook",
                    book_core::types::FileType::Comic => "comic",
                    book_core::types::FileType::Document => "document",
                    book_core::types::FileType::Junk => "junk",
                    book_core::types::FileType::Unknown => "unknown",
                };

                let record = book_core::registry::BookRecord {
                    content_hash: hash.clone(),
                    title: title.clone(),
                    author: final_author,
                    file_path: file.path.display().to_string(),
                    output_path: None,
                    size: file.size,
                    file_type: file_type_str.to_string(),
                    last_seen: 0,
                    status: "discovered".to_string(),
                };
                registry.upsert(&record)?;
                new_count += 1;
            }
        }

        if args.verbose {
            println!("📋 Registry updated: {} new files added", new_count);
            if author_count > 0 {
                println!("📋 Authors added to DB: {}", author_count);
            }
        }
    }

    if let Some(output) = args.output {
        let json = serde_json::to_string_pretty(&manifest)?;
        std::fs::write(output, json)?;
    } else {
        println!("{}", serde_json::to_string_pretty(&manifest)?);
    }

    Ok(())
}
