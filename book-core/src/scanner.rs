use crate::types::{FileType, Manifest, Stats, BookFile, FileStatus, MetadataSource};
use crate::hashing::hash_file;
use anyhow::Result;
use std::path::Path;
use walkdir::WalkDir;

pub struct Scanner {
    pub patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub min_size: u64,
    pub max_size: u64,
}

impl Default for Scanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Scanner {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                "*.mp3".to_string(),
                "*.m4a".to_string(),
                "*.flac".to_string(),
                "*.epub".to_string(),
                "*.pdf".to_string(),
                "*.mobi".to_string(),
                "*.azw3".to_string(),
                "*.cbz".to_string(),
                "*.cbr".to_string(),
            ],
            exclude_patterns: vec![
                "*.nfo".to_string(),
                "*.jpg".to_string(),
                "*.jpeg".to_string(),
                "*.png".to_string(),
                "*.gif".to_string(),
                "*.sfv".to_string(),
                "*.txt".to_string(),
                "*.log".to_string(),
                "*.bak".to_string(),
                "*.tmp".to_string(),
            ],
            min_size: 1024,
            max_size: u64::MAX,
        }
    }

    pub fn scan(&self, path: &Path) -> Result<Manifest> {
        let mut files = Vec::new();
        let mut stats = Stats::default();

        for entry in WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let metadata = entry.metadata()?;
            let size = metadata.len();

            if size < self.min_size || size > self.max_size {
                continue;
            }

            let name = path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            // Check exclude patterns
            let excluded = self.exclude_patterns.iter().any(|p| {
                if let Ok(pattern) = glob::Pattern::new(p) {
                    pattern.matches(&name)
                } else {
                    false
                }
            });

            if excluded {
                stats.add_file(&FileType::Junk, size);
                continue;
            }

            // Check include patterns
            let included = self.patterns.iter().any(|p| {
                if let Ok(pattern) = glob::Pattern::new(p) {
                    pattern.matches(&name)
                } else {
                    false
                }
            });

            if !included {
                stats.add_file(&FileType::Unknown, size);
                continue;
            }

            let ext = path.extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();

            let file_type = self.detect_type(&ext);

            // Compute content hash for dedupe/integrity
            let hash = hash_file(path).ok();

            let book_file = BookFile {
                path: path.to_path_buf(),
                name: name.clone(),
                ext: ext.clone(),
                size,
                file_type: file_type.clone(),
                metadata: None,
                inferred: None,
                chosen_source: MetadataSource::Unknown,
                content_hash: hash,
                status: FileStatus::Scanned,
                cleaned_name: None,
            };

            files.push(book_file);
            stats.add_file(&file_type, size);
        }

        Ok(Manifest {
            source: path.to_path_buf(),
            files,
            stats,
            generated: chrono::Utc::now().to_rfc3339(),
        })
    }

    fn detect_type(&self, ext: &str) -> FileType {
        match ext {
            "mp3" | "m4a" | "m4b" | "flac" | "wav" | "aac" | "ogg" => FileType::Audio,
            "epub" | "pdf" | "mobi" | "azw" | "azw3" | "ibooks" => FileType::Ebook,
            "cbz" | "cbr" | "cbt" | "cba" => FileType::Comic,
            "doc" | "docx" | "rtf" | "txt" => FileType::Document,
            "nfo" | "jpg" | "jpeg" | "png" | "gif" | "sfv" | "log" | "bak" | "tmp" => FileType::Junk,
            _ => FileType::Unknown,
        }
    }
}
