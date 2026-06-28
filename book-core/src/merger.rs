use anyhow::{Context, Result, anyhow};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs::File;
use std::io::{BufWriter, Write};
use uuid::Uuid;
use walkdir::WalkDir;

pub struct Merger {
    pub mode: MergeMode,
    pub bitrate: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum MergeMode {
    Copy,
    Encode,
}

impl Default for Merger {
    fn default() -> Self {
        Self::new()
    }
}

impl Merger {
    pub fn new() -> Self {
        Self {
            mode: MergeMode::Encode,
            bitrate: 128,
        }
    }

    pub fn check_ffmpeg(&self) -> Result<()> {
        let status = Command::new("ffmpeg")
            .arg("-version")
            .status()
            .context("Failed to run ffmpeg. Is ffmpeg installed?")?;

        if !status.success() {
            return Err(anyhow!("ffmpeg is not available. Please install ffmpeg."));
        }
        Ok(())
    }

    pub fn merge(&self, input_dir: &Path, output_name: &str, output_dir: &Path) -> Result<PathBuf> {
        // Preflight check
        self.check_ffmpeg()?;

        // Recursively find MP3s (case-insensitive)
        let mp3s = self.find_mp3s_recursive(input_dir)?;

        if mp3s.is_empty() {
            return Err(anyhow!("No MP3 files found in {}", input_dir.display()));
        }

        if mp3s.len() < 2 {
            return Err(anyhow!("Need at least 2 MP3 files (found {})", mp3s.len()));
        }

        // Create output directory
        std::fs::create_dir_all(output_dir)?;

        let output = output_dir.join(format!("{}.m4b", output_name));

        if output.exists() {
            return Ok(output);
        }

        // Create unique concat file
        let concat_file = input_dir.join(format!(".concat_{}.txt", Uuid::new_v4()));
        self.create_concat_file(&concat_file, &mp3s)?;

        // Build args
        let args = self.build_ffmpeg_args(&concat_file, &output);

        let status = Command::new("ffmpeg")
            .args(&args)
            .status()
            .context("Failed to run ffmpeg")?;

        let _ = std::fs::remove_file(&concat_file);

        if !status.success() {
            return Err(anyhow!("ffmpeg merge failed with exit code: {:?}", status.code()));
        }

        // Verify output
        if !output.exists() {
            return Err(anyhow!("Output file was not created: {}", output.display()));
        }

        let metadata = output.metadata()?;
        if metadata.len() == 0 {
            let _ = std::fs::remove_file(&output);
            return Err(anyhow!("Output file is empty (0 bytes)"));
        }

        Ok(output)
    }

    fn find_mp3s_recursive(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let mut mp3s = Vec::new();
        
        for entry in WalkDir::new(dir)
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

    fn create_concat_file(&self, path: &Path, mp3s: &[PathBuf]) -> Result<()> {
        let f = File::create(path)?;
        let mut writer = BufWriter::new(f);

        for mp3 in mp3s {
            let escaped = mp3.display().to_string().replace("'", "'\\''");
            writeln!(writer, "file '{}'", escaped)?;
        }

        writer.flush()?;
        Ok(())
    }

    fn build_ffmpeg_args(&self, concat_file: &Path, output: &Path) -> Vec<String> {
        let mut args = vec![
            "-f".to_string(),
            "concat".to_string(),
            "-safe".to_string(),
            "0".to_string(),
            "-i".to_string(),
            concat_file.display().to_string(),
            "-vn".to_string(),
        ];

        match self.mode {
            MergeMode::Copy => {
                args.push("-c:a".to_string());
                args.push("copy".to_string());
            }
            MergeMode::Encode => {
                args.push("-c:a".to_string());
                args.push("aac".to_string());
                args.push("-b:a".to_string());
                args.push(format!("{}k", self.bitrate));
            }
        }

        args.push("-movflags".to_string());
        args.push("+faststart".to_string());
        args.push("-map_metadata".to_string());
        args.push("-1".to_string());
        args.push(output.display().to_string());

        args
    }
}
