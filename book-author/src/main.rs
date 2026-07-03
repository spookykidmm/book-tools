use anyhow::Result;
use book_core::author_db::AuthorDB;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "book-author")]
#[command(about = "Manage author database")]
struct Args {
    #[arg(short, long, default_value = "~/.config/book-tools/authors.db")]
    db: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add an author
    Add {
        #[arg(help = "Author name")]
        name: String,
    },

    /// Add an alias to an author
    Alias {
        #[arg(help = "Canonical author name")]
        canonical: String,

        #[arg(help = "Alias to add")]
        alias: String,
    },

    /// Search for an author
    Search {
        #[arg(help = "Author name or alias")]
        query: String,
    },

    /// List all authors
    List,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let db_path = shellexpand::tilde(&args.db).to_string();
    let db = AuthorDB::new(&db_path)?;

    match args.command {
        Commands::Add { name } => {
            let author = db.get_or_create_author(&name)?;
            println!("✅ Added author: {}", author.canonical_name);
        }

        Commands::Alias { canonical, alias } => {
            db.add_alias(&canonical, &alias)?;
            println!("✅ Added alias '{}' to '{}'", alias, canonical);
        }

        Commands::Search { query } => {
            if let Some(author) = db.get_author_by_alias(&query)? {
                println!("📖 Found author: {}", author.canonical_name);
                println!("  Aliases: {}", author.aliases.join(", "));
            } else {
                println!("❌ No author found for: {}", query);
            }
        }

        Commands::List => {
            // Simple placeholder
            println!("📚 Authors:");
            println!("  (Use 'search' to find specific authors)");
        }
    }

    Ok(())
}
