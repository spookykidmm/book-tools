use crate::types::Metadata;
use anyhow::Result;
use std::path::Path;

pub struct MetadataExtractor;

impl MetadataExtractor {
    pub fn new() -> Self {
        Self
    }

    pub fn extract_from_file(&self, path: &Path) -> Result<Metadata> {
        let ext = path.extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();

        match ext.as_str() {
            "mp3" | "m4a" | "m4b" | "flac" | "ogg" | "wav" => {
                self.extract_audio_metadata(path)
            }
            "epub" => self.extract_epub_metadata(path),
            _ => Ok(Metadata::default()),
        }
    }

    fn extract_audio_metadata(&self, path: &Path) -> Result<Metadata> {
        use lofty::read_from_path;
        use lofty::prelude::*;

        let mut metadata = Metadata::default();

        // Try to read tags with Lofty - don't fail if no tags
        if let Ok(tagged) = read_from_path(path) {
            if let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) {
                // Extract basic metadata
                if let Some(title) = tag.title() {
                    metadata.title = Some(title.to_string());
                }

                if let Some(artist) = tag.artist() {
                    metadata.author = Some(artist.to_string());
                }

                if let Some(album) = tag.album() {
                    let album_str = album.to_string();
                    // Only set series if album differs from title
                    if let Some(ref title) = metadata.title {
                        if album_str != *title {
                            metadata.series = Some(album_str);
                        }
                    } else {
                        metadata.series = Some(album_str);
                    }
                }

                if let Some(year) = tag.year() {
                    metadata.year = Some(year as u32);
                }

                if let Some(track) = tag.track() {
                    metadata.track_number = Some(track as u32);
                }

                if let Some(genre) = tag.genre() {
                    if !genre.is_empty() {
                        metadata.tags.push(genre.to_string());
                    }
                }
            }

            // Always extract duration and bitrate from properties
            let properties = tagged.properties();
            metadata.duration = Some(properties.duration().as_secs());

            if let Some(bitrate) = properties.audio_bitrate() {
                metadata.bitrate = Some(bitrate / 1000);
            }
        } else {
            // No tags found, but we can still get duration from ffprobe if needed
            // For now, just return what we have
            eprintln!("⚠️ No tags found in {:?}, using fallback", path);
        }

        Ok(metadata)
    }

    fn extract_epub_metadata(&self, _path: &Path) -> Result<Metadata> {
        Ok(Metadata::default())
    }

    pub fn normalize(&self, metadata: &mut Metadata) -> Result<()> {
        if let Some(author) = &metadata.author {
            let mut normalized = author.to_string();
            
            if normalized.ends_with(',') {
                normalized.pop();
            }

            if let Some(pos) = normalized.find(',') {
                let last = normalized[..pos].trim();
                let first = normalized[pos + 1..].trim();
                if !first.is_empty() && !last.is_empty() {
                    normalized = format!("{} {}", first, last);
                }
            }

            metadata.author = Some(normalized);
        }

        if let Some(title) = &metadata.title {
            let mut normalized = title.to_string();
            
            if let Some(first) = normalized.chars().next() {
                normalized = format!("{}{}", first.to_uppercase(), &normalized[1..]);
            }

            metadata.title = Some(normalized);
        }

        Ok(())
    }
}
