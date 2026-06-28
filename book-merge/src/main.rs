use anyhow::{Context, Result};
use book_core::merger::{Merger, MergeMode};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "book-merge")]
#[command(about = "Merge MP3 chapters into M4B using ffmpreg")]
struct Args {
    #[arg(help = "Directory containing MP3 files")]
    input: PathBuf,

    #[arg(short, long, help = "Output directory")]
    output: Option<PathBuf>,

    #[arg(short, long, help = "Output filename (without extension)")]
    name: Option<String>,

    #[arg(short, long, default_value = "copy", help = "Mode: copy or encode")]
    mode: String,

    #[arg(long, help = "Dry run")]
    dry_run: bool,

    #[arg(short, long, help = "Verbose output")]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let output_dir = args.output.unwrap_or_else(|| {
        args.input.parent().unwrap_or(&args.input).join("merged")
    });

    if !output_dir.exists() && !args.dry_run {
        std::fs::create_dir_all(&output_dir)?;
    }

    let name = args.name.unwrap_or_else(|| {
        args.input.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });

    if args.dry_run {
        println!("[DRY RUN] Would merge MP3s from: {}", args.input.display());
        println!("  Output: {}.m4b", name);
        return Ok(());
    }

    // Find MP3 files recursively
    let mp3s = find_mp3_files(&args.input)?;
    if mp3s.is_empty() {
        return Err(anyhow::anyhow!("No MP3 files found in {}", args.input.display()));
    }

    println!("🎵 Merging {} MP3 files from:", mp3s.len());
    println!("  📂 {}", args.input.display());

    let output_path = output_dir.join(format!("{}.m4b", name));
    if output_path.exists() {
        println!("  ⏭️ Output already exists: {}", output_path.display());
        return Ok(());
    }

    // Use ffmpreg to merge
    merge_with_ffmpreg(&mp3s, &output_path, &args)?;

    println!("  ✅ Merged: {}", output_path.display());
    Ok(())
}

fn find_mp3_files(dir: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut mp3s = Vec::new();
    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext.eq_ignore_ascii_case("mp3") {
                mp3s.push(path.to_path_buf());
            }
        }
    }
    mp3s.sort();
    Ok(mp3s)
}

fn merge_with_ffmpreg(mp3s: &[PathBuf], output: &PathBuf, _args: &Args) -> Result<()> {
    // NOTE: ffmpreg's API is still evolving. 
    // This is a placeholder showing the intended flow.
    // You'll need to adapt based on the actual API at the time.
    
    use ffmpreg::container::{Mp3Reader, M4bWriter};
    use ffmpreg::codecs::{Mp3Decoder, AacEncoder};
    use ffmpreg::core::{Decoder, Encoder, Demuxer, Muxer};
    use ffmpreg::transform::{Normalize, Gain};

    let mut writer = M4bWriter::new(output)?;
    let mut decoder = Mp3Decoder::new();
    let mut encoder = AacEncoder::new(128_000);
    let mut normalize = Normalize::new(0.95);
    let mut gain = Gain::new(1.0);

    for mp3 in mp3s {
        let mut reader = Mp3Reader::from_file(mp3)?;
        while let Some(packet) = reader.read_packet()? {
            if let Some(frame) = decoder.decode(packet)? {
                let frame = gain.apply(frame)?;
                let frame = normalize.apply(frame)?;
                if let Some(out_packet) = encoder.encode(frame)? {
                    writer.write_packet(out_packet)?;
                }
            }
        }
    }

    writer.finalize()?;
    Ok(())
}
