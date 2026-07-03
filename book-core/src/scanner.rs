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
                "*.m4b".to_string(),
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

            // Compute content hash
            let hash = hash_file(path).ok();

            // Extract author from path or filename
            let (author, title) = self.extract_author_title(path, &name);

            let book_file = BookFile {
                path: path.to_path_buf(),
                name: name.clone(),
                ext: ext.clone(),
                size,
                file_type: file_type.clone(),
                metadata: None,
                inferred: Some(crate::types::Metadata {
                    author: author.clone(),
                    title: title.clone(),
                    ..Default::default()
                }),
                chosen_source: if author.is_some() { MetadataSource::Folder } else { MetadataSource::Unknown },
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

    fn extract_author_title(&self, path: &Path, filename: &str) -> (Option<String>, Option<String>) {
        // First try to extract from filename (e.g., "Author - Title.mp3")
        if let Some((author, title)) = self.parse_filename(filename) {
            return (Some(author), Some(title));
        }

        // Then try to extract from the path (e.g., "Author - Title/")
        if let Some(parent) = path.parent() {
            if let Some(dir_name) = parent.file_name() {
                let dir_str = dir_name.to_string_lossy();
                if let Some((author, title)) = self.parse_filename(&dir_str) {
                    return (Some(author), Some(title));
                }
            }
        }

        // Then try to extract from the grandparent path
        if let Some(grandparent) = path.parent().and_then(|p| p.parent()) {
            if let Some(dir_name) = grandparent.file_name() {
                let dir_str = dir_name.to_string_lossy();
                if let Some((author, title)) = self.parse_filename(&dir_str) {
                    return (Some(author), Some(title));
                }
            }
        }

        // Fallback: use the filename as the title
        (None, Some(filename.to_string()))
    }

    fn parse_filename(&self, name: &str) -> Option<(String, String)> {
        // Skip if it starts with a number (chapter marker)
        if name.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            return None;
        }

        // Try "Author - Title" pattern
        let parts: Vec<&str> = name.split(" - ").collect();
        if parts.len() >= 2 {
            // Check if the first part looks like an author name
            let first = parts[0].trim();
            let rest = parts[1..].join(" - ");
            if self.looks_like_author(first) {
                return Some((first.to_string(), rest));
            }
        }

        // Try "Title - Author" pattern
        if let Some((author, title)) = self.parse_title_author(name) {
            return Some((author, title));
        }

        None
    }

    fn looks_like_author(&self, name: &str) -> bool {
        // Skip if it's a number
        if name.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }

        // Skip if it's a single character
        if name.len() <= 2 {
            return false;
        }

        // Check for comma (Last, First)
        if name.contains(',') {
            return true;
        }

        // Check for common author name patterns
        let common_author_words = ["King", "Dick", "Asimov", "Bradbury", "Heinlein", "Tolkien"];
        for word in common_author_words {
            if name.contains(word) {
                return true;
            }
        }

        // Check if it's too short to be an author
        if name.len() < 4 {
            return false;
        }

        // Default: if it's not a number and not too short, it might be an author
        true
    }

    fn parse_title_author(&self, name: &str) -> Option<(String, String)> {
        // Look for "by Author" pattern
        if let Some(pos) = name.find(" by ") {
            let title = name[..pos].trim().to_string();
            let author = name[pos + 4..].trim().to_string();
            if !author.is_empty() && self.looks_like_author(&author) {
                return Some((author, title));
            }
        }

        // Look for "Title (Author)" pattern
        let re = regex::Regex::new(r"^(.+?)\s*\(([^)]+)\)$").unwrap();
        if let Some(caps) = re.captures(name) {
            let title = caps[1].trim().to_string();
            let author = caps[2].trim().to_string();
            if self.looks_like_author(&author) {
                return Some((author, title));
            }
        }

        None
    }
}
