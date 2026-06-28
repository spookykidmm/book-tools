use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::time::Duration;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::channel;

#[derive(Parser)]
#[command(name = "book-watch")]
#[command(about = "Watch for new books and auto-process")]
struct Args {
    #[arg(help = "Directory to watch")]
    watch: PathBuf,

    #[arg(short, long, help = "Debounce time in seconds")]
    debounce: Option<u64>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let debounce = Duration::from_secs(args.debounce.unwrap_or(10));

    println!("👁️  Watching: {}", args.watch.display());
    println!("⏱️  Debounce: {} seconds", debounce.as_secs());
    println!("Press Ctrl+C to stop");

    let (tx, rx) = channel();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                    let _ = tx.send(());
                }
            }
        },
        Config::default()
    )?;

    watcher.watch(&args.watch, RecursiveMode::Recursive)?;

    let mut last_event = std::time::Instant::now();

    for _ in rx {
        let now = std::time::Instant::now();

        if now.duration_since(last_event) > debounce {
            last_event = now;
            println!("📦 New files detected! Processing...");

            let output = args.watch.join("ready_for_calibre");

            let status = std::process::Command::new("book-pretty")
                .arg(&args.watch)
                .arg("--output")
                .arg(&output)
                .status();

            if let Ok(status) = status {
                if status.success() {
                    println!("✅ Auto-process complete!");
                } else {
                    eprintln!("❌ Auto-process failed");
                }
            }
        }
    }

    Ok(())
}
