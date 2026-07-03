use crate::types::{BookUnit, BookStatus, FileType};
use crate::hashing::hash_file;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct UnitBuilder {
    pub min_mp3s: usize,
    pub max_depth: usize,
}

impl Default for UnitBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl UnitBuilder {
    pub fn new() -> Self {
        Self {
            min_mp3s: 2,
            max_depth: 3,
        }
    }

    pub fn group_files(&self, files: &[PathBuf]) -> Result<Vec<BookUnit>> {
        let mut units = Vec::new();
        let mut mp3_groups: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

        // 1. Group MP3s by directory
        for file in files {
            if let Some(ext) = file.extension() {
                if ext == "mp3" {
                    if let Some(parent) = file.parent() {
                        mp3_groups.entry(parent.to_path_buf())
                            .or_insert_with(Vec::new)
                            .push(file.clone());
                    }
                }
            }
        }

        // 2. Create BookUnits from MP3 groups
        for (dir, mp3s) in mp3_groups {
            if mp3s.len() >= self.min_mp3s {
                let unit = self.create_audio_unit(&dir, &mp3s)?;
                units.push(unit);
            } else {
                // Small groups or single MP3s
                for mp3 in mp3s {
                    let unit = self.create_single_file_unit(&mp3)?;
                    units.push(unit);
                }
            }
        }

        // 3. Handle non-MP3 files (ebooks, etc.)
        for file in files {
            if let Some(ext) = file.extension() {
                if ext != "mp3" {
                    let unit = self.create_single_file_unit(file)?;
                    units.push(unit);
                }
            }
        }

        Ok(units)
    }

    fn create_audio_unit(&self, dir: &Path, mp3s: &[PathBuf]) -> Result<BookUnit> {
        let id = Uuid::new_v4().to_string();
        let name = dir.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Compute combined hash of all MP3s
        let mut hasher = blake3::Hasher::new();
        for mp3 in mp3s {
            if let Ok(hash) = hash_file(mp3) {
                hasher.update(hash.as_bytes());
            }
        }
        let work_hash = hasher.finalize().to_hex().to_string();

        Ok(BookUnit {
            id,
            title: Some(name),
            author: None,
            series: None,
            source_files: mp3s.to_vec(),
            merged_output: None,
            file_type: FileType::Audio,
            work_hash: Some(work_hash),
            status: BookStatus::Discovered,
            metadata: None,
        })
    }

    fn create_single_file_unit(&self, file: &Path) -> Result<BookUnit> {
        let id = Uuid::new_v4().to_string();
        let name = file.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let ext = file.extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();

        let file_type = match ext.as_str() {
            "mp3" | "m4a" | "m4b" | "flac" => FileType::Audio,
            "epub" | "pdf" | "mobi" | "azw3" => FileType::Ebook,
            "cbz" | "cbr" => FileType::Comic,
            _ => FileType::Unknown,
        };

        let hash = hash_file(file).ok();
        let work_hash = hash.clone();

        Ok(BookUnit {
            id,
            title: Some(name),
            author: None,
            series: None,
            source_files: vec![file.to_path_buf()],
            merged_output: None,
            file_type,
            work_hash,
            status: BookStatus::Discovered,
            metadata: None,
        })
    }
}
