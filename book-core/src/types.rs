use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub source: PathBuf,
    pub files: Vec<BookFile>,
    pub stats: Stats,
    pub generated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookFile {
    pub path: PathBuf,
    pub name: String,
    pub ext: String,
    pub size: u64,
    pub file_type: FileType,
    pub metadata: Option<Metadata>,
    pub inferred: Option<Metadata>,
    pub chosen_source: MetadataSource,
    pub content_hash: Option<String>,
    pub status: FileStatus,
    pub cleaned_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileType {
    Audio,
    Ebook,
    Comic,
    Document,
    Unknown,
    Junk,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Metadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub narrator: Option<String>,
    pub series: Option<String>,
    pub series_index: Option<u32>,
    pub disc_number: Option<u32>,
    pub track_number: Option<u32>,
    pub year: Option<u32>,
    pub publisher: Option<String>,
    pub isbn: Option<String>,
    pub language: Option<String>,
    pub tags: Vec<String>,
    pub cover: Option<PathBuf>,
    pub bitrate: Option<u32>,
    pub duration: Option<u64>,
    pub chapters: Vec<Chapter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub title: String,
    pub start: u64,
    pub end: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MetadataSource {
    Embedded,   // From ID3 tags (highest confidence)
    Mapping,    // From configured mapping
    Folder,     // From folder structure
    Filename,   // From filename heuristics
    Unknown,    // No source
}

impl MetadataSource {
    pub fn confidence(&self) -> f32 {
        match self {
            MetadataSource::Embedded => 1.0,
            MetadataSource::Mapping => 0.9,
            MetadataSource::Folder => 0.75,
            MetadataSource::Filename => 0.5,
            MetadataSource::Unknown => 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileStatus {
    Pending,
    Scanned,
    Cleaned,
    Merged,
    Organized,
    Verified,
    Quarantined,
    Failed,
}

impl Default for FileStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Stats {
    pub total_files: usize,
    pub total_size: u64,
    pub audio_files: usize,
    pub ebook_files: usize,
    pub comic_files: usize,
    pub document_files: usize,
    pub junk_files: usize,
    pub unknown_files: usize,
    pub duplicates: usize,
}

impl Stats {
    pub fn add_file(&mut self, file_type: &FileType, size: u64) {
        self.total_files += 1;
        self.total_size += size;
        match file_type {
            FileType::Audio => self.audio_files += 1,
            FileType::Ebook => self.ebook_files += 1,
            FileType::Comic => self.comic_files += 1,
            FileType::Document => self.document_files += 1,
            FileType::Junk => self.junk_files += 1,
            FileType::Unknown => self.unknown_files += 1,
        }
    }
}
