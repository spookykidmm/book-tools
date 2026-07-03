use anyhow::Result;
use book_core::decision::{Decision, DecisionEngine, DedupePolicy};
use book_core::registry::Registry;
use book_core::scanner::Scanner;
use clap::{Parser, Subcommand};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "book-decision")]
#[command(about = "Decision engine for book processing")]
struct Args {
    #[arg(short, long, default_value = "~/.config/book-tools/registry.db")]
    db: String,

    #[arg(short, long)]
    path: Option<PathBuf>,

    #[arg(short, long, default_value = "largest")]
    dedupe: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze a directory and make decisions
    Analyze {
        #[arg(help = "Directory to analyze")]
        path: PathBuf,

        #[arg(long)]
        verbose: bool,
    },

    /// Evaluate duplicates in the registry
    Evaluate,

    /// Verify file integrity
    Verify {
        #[arg(help = "Hash to verify")]
        hash: String,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();
    let db_path = shellexpand::tilde(&args.db).to_string();
    let registry = Registry::new(&db_path)?;

    let policy = match args.dedupe.as_str() {
        "largest" => DedupePolicy::KeepLargest,
        "bitrate" => DedupePolicy::KeepBestBitrate,
        "first" => DedupePolicy::KeepFirst,
        "all" => DedupePolicy::KeepAll,
        _ => DedupePolicy::KeepLargest,
    };

    let engine = DecisionEngine::new(policy);

    match args.command {
        Commands::Analyze { path, verbose } => {
            let scanner = Scanner::new();
            let manifest = scanner.scan(&path)?;

            println!("📊 Analyzing {} files", manifest.files.len());

            let mut decisions = Vec::new();
            let mut run_seen = HashSet::new();

            for file in &manifest.files {
                let hash = file.content_hash.clone().unwrap_or_default();
                let registry_record = if !hash.is_empty() {
                    registry.get(&hash)?
                } else {
                    None
                };

                // Real output_exists check
                let output_exists = registry_record
                    .as_ref()
                    .and_then(|r| r.output_path.as_ref())
                    .map(|p| std::path::Path::new(p).exists())
                    .unwrap_or(false);

                let ctx = book_core::decision::DecisionContext {
                    file: file.clone(),
                    registry_record,
                    run_seen_hashes: run_seen.clone(),
                    output_exists,
                };

                // Decision BEFORE insert
                let decision = engine.decide(&ctx)?;
                let decision_clone = decision.clone();

                // Insert AFTER decision
                if let Some(ref h) = file.content_hash {
                    run_seen.insert(h.clone());
                }

                decisions.push((file.path.clone(), decision));

                if verbose {
                    println!("  {}: {:?}", file.path.display(), decision_clone);
                }
            }

            let process_count = decisions.iter()
                .filter(|(_, d)| matches!(d, Decision::Process))
                .count();
            let skip_count = decisions.iter()
                .filter(|(_, d)| matches!(d, Decision::Skip))
                .count();
            let dup_count = decisions.iter()
                .filter(|(_, d)| matches!(d, Decision::Duplicate { .. }))
                .count();
            let quarantine_count = decisions.iter()
                .filter(|(_, d)| matches!(d, Decision::Quarantine))
                .count();

            println!("\n📊 Decision Summary:");
            println!("  Process: {}", process_count);
            println!("  Skip: {}", skip_count);
            println!("  Duplicates: {}", dup_count);
            println!("  Quarantine: {}", quarantine_count);
        }

        Commands::Evaluate => {
            let duplicates = registry.get_duplicates()?;
            println!("♻️ Evaluating Duplicate Groups");
            println!("=============================");

            if duplicates.is_empty() {
                println!("✅ No duplicates found!");
                return Ok(());
            }

            for (author, records) in &duplicates {
                println!("\n📖 {} ({} copies)", author, records.len());
                let kept = engine.evaluate_duplicate_group(records.clone())?;
                if let Some(first) = kept.first() {
                    let title = first.title.as_deref().unwrap_or("Unknown");
                    println!("  ✅ Keeping: {}", title);
                    for record in &kept {
                        let t = record.title.as_deref().unwrap_or("Unknown");
                        println!("    - {} [{}]", t, &record.content_hash[..8]);
                    }
                }
            }
        }

        Commands::Verify { hash } => {
            let record = registry.get(&hash)?;
            if let Some(record) = record {
                let is_valid = engine.verify_file_integrity(&hash, &record.file_path)?;
                if is_valid {
                    println!("✅ File is valid");
                } else {
                    println!("❌ File is corrupt or mismatched");
                }
            } else {
                println!("❌ Hash not found in registry");
            }
        }
    }

    Ok(())
}
