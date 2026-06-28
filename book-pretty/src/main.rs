use anyhow::Result;
use book_core::{
    scanner::Scanner,
    cleaner::Cleaner,
    merger::Merger,
    organizer::{Organizer, OrganizationStructure, DuplicateHandling},
    types::Metadata,
};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "book-pretty")]
#[command(about = "One-command weekly book cleanup")]
struct Args {
    #[arg(help = "Source directory")]
    source: PathBuf,

    #[arg(short, long, help = "Output directory")]
    output: Option<PathBuf>,

    #[arg(long, help = "Dry run")]
    dry_run: bool,

    #[arg(short, long, help = "Verbose output")]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let output_path = args.output.clone().unwrap_or_else(|| {
        args.source.join("ready_for_calibre")
    });

    println!("📚 PRETTY BOOKS (SAFE MODE)");
    println!("===========================");
    println!("Source: {}", args.source.display());
    println!("Output: {}", output_path.display());
    println!("Mode: {}", if args.dry_run { "DRY RUN" } else { "APPLY" });
    println!("⚠️  Files are COPIED, not moved (safety first!)");
    println!();

    // Step 1: Scan
    println!("🔍 Scanning...");
    let scanner = Scanner::new();
    let mut manifest = scanner.scan(&args.source)?;

    if manifest.files.is_empty() {
        println!("✨ No files found.");
        return Ok(());
    }

    println!("📊 Found {} files", manifest.files.len());
    println!();

    // Step 2: Clean - extract metadata from filenames and store in metadata field
    println!("🧹 Cleaning filenames...");
    let cleaner = Cleaner::new();
    for file in &mut manifest.files {
        let clean = cleaner.clean_filename(&file.name);
        file.cleaned_name = Some(clean);
        
        // Extract metadata from filename
        let (author, title, narrator, series) = cleaner.extract_metadata(file);
        
        // Build inferred metadata
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
        
        // Store as inferred metadata
        file.inferred = Some(inferred);
        file.chosen_source = book_core::types::MetadataSource::Filename;
        
        // If no metadata exists yet, use inferred as primary
        if file.metadata.is_none() {
            file.metadata = file.inferred.clone();
        }
    }
    println!();

    // Step 3: Merge audio (only if MP3s found)
    let audio_files: Vec<_> = manifest.files.iter()
        .filter(|f| f.file_type == book_core::types::FileType::Audio)
        .collect();

    if !audio_files.is_empty() && !args.dry_run {
        println!("🎵 Merging audiobooks...");
        let audio_output = output_path.join("Audiobooks");
        std::fs::create_dir_all(&audio_output)?;
        let merger = Merger::new();
        let mut merged = 0;

        for file in audio_files {
            if let Some(parent) = file.path.parent() {
                let name = file.cleaned_name.as_ref().unwrap_or(&file.name);
                let clean_name = name.replace(".mp3", "").replace(".m4b", "");
                if let Ok(result) = merger.merge(parent, &clean_name, &audio_output) {
                    merged += 1;
                    if args.verbose {
                        println!("  ✅ {}", result.display());
                    }
                }
            }
        }
        println!("  Merged {} audiobooks", merged);
        println!();
    } else if args.dry_run && !audio_files.is_empty() {
        println!("  [DRY RUN] Would merge {} audiobooks", audio_files.len());
        println!();
    }

    // Step 4: Organize (COPY mode, safe!)
    println!("📂 Organizing files (copying, not moving)...");
    if args.dry_run {
        println!("  [DRY RUN] Would organize {} files", manifest.files.len());
    } else {
        let output_path_clone = output_path.clone();
        let mut organizer = Organizer::new(output_path_clone);
        organizer.move_files = false;
        organizer.structure = OrganizationStructure::AuthorTitle;
        organizer.handle_duplicates = DuplicateHandling::Rename;
        organizer.dry_run = false;

        let organized = organizer.organize(&mut manifest)?;
        println!("  ✅ Organized {} files", organized.len());
    }
    println!();

    println!("✨ Done!");
    println!("📂 Output: {}", output_path.display());
    println!("⚠️  Original files were COPIED, not moved.");
    println!("   Your source files are still safe!");

    Ok(())
}
