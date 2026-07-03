use anyhow::Result;
use book_core::registry::Registry;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "book-registry")]
#[command(about = "Manage the global book registry")]
struct Args {
    #[arg(short, long, default_value = "~/.config/book-tools/registry.db")]
    db: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show registry statistics
    Stats,

    /// List all processed books
    List {
        #[arg(short, long)]
        author: Option<String>,

        #[arg(short, long)]
        status: Option<String>,
    },

    /// Find duplicates by author/title
    Duplicates,

    /// Show orphans (files in registry but missing from filesystem)
    Orphans,

    /// Verify integrity of all files
    Verify,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let db_path = shellexpand::tilde(&args.db).to_string();
    let registry = Registry::new(&db_path)?;

    match args.command {
        Commands::Stats => {
            let count = registry.count()?;
            println!("📊 Registry Statistics");
            println!("======================");
            println!("Total books: {}", count);
        }

        Commands::List { author, status } => {
            let records = registry.get_all()?;
            println!("📚 All Books");
            println!("============");

            for record in records {
                if let Some(ref a) = author {
                    if record.author.as_ref() != Some(a) {
                        continue;
                    }
                }
                if let Some(ref s) = status {
                    if &record.status != s {
                        continue;
                    }
                }

                let title = record.title.as_deref().unwrap_or("Unknown");
                let author = record.author.as_deref().unwrap_or("Unknown");
                println!(
                    "  {} - {} [{}] ({})",
                    author,
                    title,
                    &record.content_hash[..8],
                    record.status
                );
            }
        }

        Commands::Duplicates => {
            let duplicates = registry.get_duplicates()?;
            println!("♻️ Duplicate Groups");
            println!("==================");

            if duplicates.is_empty() {
                println!("✅ No duplicates found!");
                return Ok(());
            }

            for (author, records) in &duplicates {
                println!("\n📖 {} ({} copies)", author, records.len());
                for record in records {
                    let title = record.title.as_deref().unwrap_or("Unknown");
                    println!("    - {} [{}]", title, &record.content_hash[..8]);
                }
            }
        }

        Commands::Orphans => {
            println!("🔍 Checking for orphans...");
            println!("⚠️  Orphan check not yet implemented");
            println!("   (This will check if files in registry still exist)");
        }

        Commands::Verify => {
            println!("🔐 Verifying all files...");
            println!("⚠️  Verification not yet implemented");
            println!("   (This will check content_hash matches actual file)");
        }
    }

    Ok(())
}
