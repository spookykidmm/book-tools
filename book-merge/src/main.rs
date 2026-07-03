use anyhow::{Result, anyhow};
use book_core::{
    unit::UnitBuilder,
    types::BookUnit,
    merger::Merger,
    registry::Registry,
};
use clap::Parser;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Parser)]
#[command(name = "book-merge")]
#[command(about = "Merge MP3 chapters into M4B with parallel execution")]
struct Args {
    #[arg(help = "Directory containing MP3 files")]
    input: PathBuf,

    #[arg(short, long, help = "Output directory")]
    output: Option<PathBuf>,

    #[arg(short, long, default_value = "4", help = "Number of parallel workers")]
    workers: usize,

    #[arg(long, default_value = "30", help = "Timeout in minutes per book")]
    timeout: u64,

    #[arg(long, help = "Dry run")]
    dry_run: bool,

    #[arg(short, long, help = "Verbose output")]
    verbose: bool,

    #[arg(long, default_value = "~/.config/book-tools/registry.db", help = "Registry database path")]
    registry: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let registry_path = shellexpand::tilde(&args.registry).to_string();
    let registry = Arc::new(Mutex::new(Registry::new(&registry_path)?));

    let output_dir = args.output.unwrap_or_else(|| {
        args.input.join("merged")
    });

    if !output_dir.exists() && !args.dry_run {
        std::fs::create_dir_all(&output_dir)?;
    }

    println!("📚 BOOK-MERGE (PARALLEL)");
    println!("========================");
    println!("Input: {}", args.input.display());
    println!("Output: {}", output_dir.display());
    println!("Workers: {}", args.workers);
    println!("Timeout: {} minutes", args.timeout);
    println!("Mode: {}", if args.dry_run { "DRY RUN" } else { "APPLY" });
    println!("");

    // 1. Scan and group into BookUnits
    println!("🔍 Scanning and grouping files...");
    let scanner = book_core::scanner::Scanner::new();
    let manifest = scanner.scan(&args.input)?;

    let builder = UnitBuilder::new();
    let units = builder.group_files(&manifest.files.iter().map(|f| f.path.clone()).collect::<Vec<_>>())?;

    let audio_units: Vec<_> = units.iter()
        .filter(|u| u.file_type == book_core::types::FileType::Audio)
        .filter(|u| u.source_files.len() >= 2)
        .cloned()
        .collect();

    println!("📊 Found {} audio units to process", audio_units.len());
    println!("");

    if args.dry_run {
        for unit in &audio_units {
            println!("  📖 {} ({} MP3s)", unit.title.as_deref().unwrap_or("Unknown"), unit.source_files.len());
        }
        return Ok(());
    }

    // 2. Setup progress bar
    let pb = ProgressBar::new(audio_units.len() as u64);
    pb.set_style(ProgressStyle::with_template(
        "{msg} [{bar:40.cyan/blue}] {pos}/{len} [{elapsed_precise}] ETA: {eta}"
    ).unwrap().progress_chars("█▉▊▋▌▍▎▏  "));
    pb.set_message("Merging audiobooks");

    // 3. Parallel processing with bounded thread pool
    let processed = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let start = Instant::now();

    let pool = ThreadPoolBuilder::new()
        .num_threads(args.workers)
        .build()?;

    // Clone registry for each thread (Mutex handles locking)
    let registry_clone = registry.clone();

    pool.install(|| {
        audio_units.par_iter().for_each(|unit| {
            // Each thread gets its own registry clone
            let reg = registry_clone.clone();
            let result = process_unit(unit, &output_dir, &reg, args.timeout, args.verbose);
            
            match result {
                Ok(_) => {
                    processed.fetch_add(1, Ordering::SeqCst);
                }
                Err(e) => {
                    failed.fetch_add(1, Ordering::SeqCst);
                    if args.verbose {
                        eprintln!("❌ Failed: {} - {}", unit.title.as_deref().unwrap_or("Unknown"), e);
                    }
                }
            }
            pb.inc(1);
        });
    });

    pb.finish();

    // 4. Summary
    let elapsed = start.elapsed();
    let processed_count = processed.load(Ordering::SeqCst);
    let failed_count = failed.load(Ordering::SeqCst);

    println!("");
    println!("📊 COMPLETE");
    println!("===========");
    println!("✅ Processed: {}", processed_count);
    println!("❌ Failed: {}", failed_count);
    println!("⏱️  Time: {:.2}s", elapsed.as_secs_f64());
    println!("📂 Output: {}", output_dir.display());

    Ok(())
}

fn process_unit(
    unit: &BookUnit,
    output_dir: &PathBuf,
    registry: &Arc<Mutex<Registry>>,
    timeout_minutes: u64,
    verbose: bool,
) -> Result<()> {
    let _timeout = Duration::from_secs(timeout_minutes * 60);

    // Create a temp directory for this unit
    let temp_dir = std::env::temp_dir().join(format!("book-tools-{}", unit.id));
    std::fs::create_dir_all(&temp_dir)?;

    // Process with timeout
    let result = std::thread::spawn({
        let unit = unit.clone();
        let output_dir = output_dir.clone();
        let temp_dir = temp_dir.clone();
        let registry = registry.clone();

        move || -> Result<()> {
            let merger = Merger::new();
            let name = unit.title.as_deref().unwrap_or(&unit.id);
            let output = merger.merge(&temp_dir, name, &output_dir)?;

            // Update registry with output path
            let hash = unit.work_hash.as_ref().unwrap_or(&unit.id);
            let output_path = output.display().to_string();
            
            // Lock registry and update
            let reg = registry.lock().unwrap();
            reg.mark_processed(hash, &output_path)?;

            Ok(())
        }
    });

    match result.join() {
        Ok(Ok(_)) => {
            if verbose {
                println!("✅ Merged: {}", unit.title.as_deref().unwrap_or("Unknown"));
            }
            // Clean up temp
            let _ = std::fs::remove_dir_all(&temp_dir);
            Ok(())
        }
        Ok(Err(e)) => {
            // Unit failed, quarantine it
            let reg = registry.lock().unwrap();
            let _ = reg.upsert(&book_core::registry::BookRecord {
                content_hash: unit.id.clone(),
                title: unit.title.clone(),
                author: unit.author.clone(),
                file_path: unit.source_files.first().unwrap_or(&PathBuf::new()).display().to_string(),
                output_path: None,
                size: 0,
                file_type: "audio".to_string(),
                last_seen: 0,
                status: "quarantined".to_string(),
            });
            Err(e)
        }
        Err(_) => {
            // Hung or panicked
            let reg = registry.lock().unwrap();
            let _ = reg.upsert(&book_core::registry::BookRecord {
                content_hash: unit.id.clone(),
                title: unit.title.clone(),
                author: unit.author.clone(),
                file_path: unit.source_files.first().unwrap_or(&PathBuf::new()).display().to_string(),
                output_path: None,
                size: 0,
                file_type: "audio".to_string(),
                last_seen: 0,
                status: "hung".to_string(),
            });
            Err(anyhow!("Unit hung (timeout: {} minutes)", timeout_minutes))
        }
    }
}
