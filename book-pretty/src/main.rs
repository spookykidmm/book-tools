use anyhow::Result;
use book_core::{
    scanner::Scanner,
    cleaner::Cleaner,
    merger::Merger,
    organizer::{Organizer, OrganizationStructure, DuplicateHandling},
    registry::{Registry, BookRecord},
    decision::{DecisionEngine, DecisionContext, DedupePolicy, Decision},
    types::Metadata,
};
use clap::Parser;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Parser)]
#[command(name = "book-pretty")]
#[command(about = "One-command weekly book cleanup with Decision Engine")]
struct Args {
    #[arg(help = "Source directory")]
    source: PathBuf,

    #[arg(short, long, help = "Output directory")]
    output: Option<PathBuf>,

    #[arg(long, help = "Dry run")]
    dry_run: bool,

    #[arg(short, long, help = "Verbose output")]
    verbose: bool,

    #[arg(long, default_value = "~/.config/book-tools/registry.db", help = "Registry database path")]
    registry: String,

    #[arg(long, default_value = "largest", help = "Dedupe policy: largest, bitrate, first, all")]
    dedupe: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let registry_path = shellexpand::tilde(&args.registry).to_string();
    let registry = Registry::new(&registry_path)?;

    let output_path = args.output.clone().unwrap_or_else(|| {
        args.source.join("ready_for_calibre")
    });

    let policy = match args.dedupe.as_str() {
        "largest" => DedupePolicy::KeepLargest,
        "bitrate" => DedupePolicy::KeepBestBitrate,
        "first" => DedupePolicy::KeepFirst,
        "all" => DedupePolicy::KeepAll,
        _ => DedupePolicy::KeepLargest,
    };

    let policy_clone = policy.clone();
    let engine = DecisionEngine::new(policy);

    println!("📚 PRETTY BOOKS (DECISION ENGINE)");
    println!("=================================");
    println!("Source: {}", args.source.display());
    println!("Output: {}", output_path.display());
    println!("Registry: {}", registry_path);
    println!("Mode: {}", if args.dry_run { "DRY RUN" } else { "APPLY" });
    println!("Dedupe Policy: {:?}", policy_clone);
    println!("");

    // Step 1: Scan
    println!("🔍 Scanning...");
    let scanner = Scanner::new();
    let mut manifest = scanner.scan(&args.source)?;

    if manifest.files.is_empty() {
        println!("✨ No files found.");
        return Ok(());
    }

    println!("📊 Found {} files", manifest.files.len());
    println!("");

    // Step 2: Clean and Extract Metadata
    println!("🧹 Cleaning filenames and extracting metadata...");
    let cleaner = Cleaner::new();
    for file in &mut manifest.files {
        let clean = cleaner.clean_filename(&file.name);
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

        file.inferred = Some(inferred);
        file.chosen_source = book_core::types::MetadataSource::Filename;

        if file.metadata.is_none() {
            file.metadata = file.inferred.clone();
        }
    }
    println!("");

    // Step 3: Upsert ALL files to registry
    println!("📋 Updating registry with all discovered files...");
    let mut new_count = 0;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    for file in &manifest.files {
        if let Some(hash) = &file.content_hash {
            let existing = registry.get(hash)?;

            if existing.is_none() {
                let (title, author) = if let Some(meta) = &file.metadata {
                    (meta.title.clone(), meta.author.clone())
                } else if let Some(inferred) = &file.inferred {
                    (inferred.title.clone(), inferred.author.clone())
                } else {
                    (None, None)
                };

                let file_type_str = match file.file_type {
                    book_core::types::FileType::Audio => "audio",
                    book_core::types::FileType::Ebook => "ebook",
                    book_core::types::FileType::Comic => "comic",
                    book_core::types::FileType::Document => "document",
                    book_core::types::FileType::Junk => "junk",
                    book_core::types::FileType::Unknown => "unknown",
                };

                let record = BookRecord {
                    content_hash: hash.clone(),
                    title,
                    author,
                    file_path: file.path.display().to_string(),
                    output_path: None,
                    size: file.size,
                    file_type: file_type_str.to_string(),
                    last_seen: now,
                    status: "discovered".to_string(),
                };
                registry.upsert(&record)?;
                new_count += 1;
            }
        }
    }

    println!("  ✅ Registry updated: {} new files added", new_count);
    println!("");

    // Step 4: Decision Engine - Decide what to do with each file
    println!("🧠 Decision Engine analyzing {} files...", manifest.files.len());

    let mut decisions: Vec<(PathBuf, Decision)> = Vec::new();
    let mut run_seen_hashes = HashSet::new();
    let mut files_to_merge = Vec::new();
    let mut files_to_organize = Vec::new();

    for file in &manifest.files {
        let hash = file.content_hash.clone().unwrap_or_default();
        let registry_record = if !hash.is_empty() {
            registry.get(&hash)?
        } else {
            None
        };

        let output_exists = registry_record
            .as_ref()
            .and_then(|r| r.output_path.as_ref())
            .map(|p| std::path::Path::new(p).exists())
            .unwrap_or(false);

        let ctx = DecisionContext {
            file: file.clone(),
            registry_record,
            run_seen_hashes: run_seen_hashes.clone(),
            output_exists,
        };

        let decision = engine.decide(&ctx)?;
        let decision_clone = decision.clone();

        if let Some(ref h) = file.content_hash {
            run_seen_hashes.insert(h.clone());
        }

        decisions.push((file.path.clone(), decision));

        match decision_clone {
            Decision::Process => {
                files_to_merge.push(file);
            }
            Decision::Reprocess => {
                // If output doesn't exist, we need to organize it
                if !output_exists {
                    files_to_organize.push(file);
                }
            }
            Decision::Skip => {
                // Already processed, do nothing
            }
            Decision::Duplicate { .. } => {
                // Handle duplicates
            }
            Decision::Quarantine => {
                // Handle quarantine
            }
        }
    }

    // Show decisions
    let process_count = decisions.iter().filter(|(_, d)| matches!(d, Decision::Process)).count();
    let skip_count = decisions.iter().filter(|(_, d)| matches!(d, Decision::Skip)).count();
    let dup_count = decisions.iter().filter(|(_, d)| matches!(d, Decision::Duplicate { .. })).count();
    let quarantine_count = decisions.iter().filter(|(_, d)| matches!(d, Decision::Quarantine)).count();
    let reprocess_count = decisions.iter().filter(|(_, d)| matches!(d, Decision::Reprocess)).count();

    println!("\n📊 Decision Summary:");
    println!("  Process: {}", process_count);
    println!("  Skip: {}", skip_count);
    println!("  Reprocess: {}", reprocess_count);
    println!("  Duplicates: {}", dup_count);
    println!("  Quarantine: {}", quarantine_count);

    if args.verbose {
        for (path, decision) in &decisions {
            println!("  {}: {:?}", path.display(), decision);
        }
    }
    println!("");

    // Step 5: Merge Audio (only files that need processing)
    if !files_to_merge.is_empty() && !args.dry_run {
        println!("🎵 Merging {} audiobooks...", files_to_merge.len());
        let audio_output = output_path.join("Audiobooks");
        std::fs::create_dir_all(&audio_output)?;
        let merger = Merger::new();
        let mut merged = 0;

        for file in files_to_merge {
            if let Some(parent) = file.path.parent() {
                let name = file.cleaned_name.as_ref().unwrap_or(&file.name);
                let clean_name = name.replace(".mp3", "").replace(".m4b", "");
                if let Ok(result) = merger.merge(parent, &clean_name, &audio_output) {
                    merged += 1;
                    if args.verbose {
                        println!("  ✅ {}", result.display());
                    }
                    if let Some(hash) = &file.content_hash {
                        let output_path_str = result.display().to_string();
                        registry.mark_processed(hash, &output_path_str)?;
                    }
                }
            }
        }
        println!("  Merged {} audiobooks", merged);
        println!();
    } else if args.dry_run && !files_to_merge.is_empty() {
        println!("  [DRY RUN] Would merge {} audiobooks", files_to_merge.len());
        println!();
    }

    // Step 6: Organize (COPY mode, safe!) and update registry with output paths
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

        // Update registry with output paths for organized files
        println!("  📋 Updating registry with output paths...");
        for file in &manifest.files {
            if let Some(hash) = &file.content_hash {
                // Find the organized file path
                if let Some(dest) = organized.iter().find(|p| {
                    // Check if this path corresponds to the file
                    p.file_name().and_then(|n| n.to_str())
                        .map(|n| n.contains(&file.name.replace(".m4b", "")))
                        .unwrap_or(false)
                }) {
                    let output_path_str = dest.display().to_string();
                    registry.mark_processed(hash, &output_path_str)?;
                }
            }
        }
        println!("  ✅ Registry updated with output paths");
    }
    println!();

    println!("✨ Done!");
    println!("📂 Output: {}", output_path.display());
    println!("⚠️  Original files were COPIED, not moved.");
    println!("   Your source files are still safe!");

    Ok(())
}
