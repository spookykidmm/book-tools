use crate::types::{BookFile, FileType, Manifest, FileStatus};
use crate::hashing::hash_file;
use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct Organizer {
    pub output_dir: PathBuf,
    pub structure: OrganizationStructure,
    pub move_files: bool,
    pub handle_duplicates: DuplicateHandling,
    pub dry_run: bool,
    pub verify: bool,
}

#[derive(Debug, Clone)]
pub enum OrganizationStructure {
    AuthorTitle,
    AuthorSeriesTitle,
    Title,
}

#[derive(Debug, Clone)]
pub enum DuplicateHandling {
    Rename,
    Skip,
    Overwrite,
}

impl Organizer {
    pub fn new(output_dir: PathBuf) -> Self {
        Self {
            output_dir,
            structure: OrganizationStructure::AuthorTitle,
            move_files: false,
            handle_duplicates: DuplicateHandling::Rename,
            dry_run: false,
            verify: true,
        }
    }

    pub fn organize(&self, manifest: &mut Manifest) -> Result<Vec<PathBuf>> {
        let mut organized = Vec::new();

        for file in &mut manifest.files {
            if file.file_type == FileType::Junk {
                continue;
            }

            if !file.path.exists() {
                eprintln!("⚠️ Source missing: {}", file.path.display());
                file.status = FileStatus::Failed;
                continue;
            }

            let dest = self.destination_path(file)?;

            if dest.exists() {
                match self.handle_duplicates {
                    DuplicateHandling::Rename => {
                        let dest = self.rename_duplicate(&dest)?;
                        if self.dry_run {
                            println!("  [DRY RUN] Would copy: {} → {}", file.path.display(), dest.display());
                            organized.push(dest);
                            continue;
                        }
                        self.atomic_copy(file, &dest)?;
                        if self.verify {
                            self.verify_copy(file, &dest)?;
                            file.status = FileStatus::Verified;
                        } else {
                            file.status = FileStatus::Organized;
                        }
                        organized.push(dest);
                    }
                    DuplicateHandling::Skip => continue,
                    DuplicateHandling::Overwrite => {
                        if self.dry_run {
                            println!("  [DRY RUN] Would overwrite: {}", dest.display());
                            organized.push(dest);
                            continue;
                        }
                        fs::remove_file(&dest)?;
                        self.atomic_copy(file, &dest)?;
                        if self.verify {
                            self.verify_copy(file, &dest)?;
                            file.status = FileStatus::Verified;
                        } else {
                            file.status = FileStatus::Organized;
                        }
                        organized.push(dest);
                    }
                }
            } else {
                if self.dry_run {
                    println!("  [DRY RUN] Would copy: {} → {}", file.path.display(), dest.display());
                    organized.push(dest);
                    continue;
                }
                self.atomic_copy(file, &dest)?;
                if self.verify {
                    self.verify_copy(file, &dest)?;
                    file.status = FileStatus::Verified;
                } else {
                    file.status = FileStatus::Organized;
                }
                organized.push(dest);
            }
        }

        Ok(organized)
    }

    fn atomic_copy(&self, file: &BookFile, dest: &Path) -> Result<()> {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        let temp_dest = dest.parent().unwrap().join(format!(".tmp_{}", Uuid::new_v4()));
        fs::copy(&file.path, &temp_dest)?;

        if !temp_dest.exists() {
            return Err(anyhow!("Temp copy failed: {}", temp_dest.display()));
        }

        fs::rename(&temp_dest, dest)?;

        if !dest.exists() {
            return Err(anyhow!("Destination not created: {}", dest.display()));
        }

        let metadata = dest.metadata()?;
        if metadata.len() == 0 {
            fs::remove_file(dest)?;
            return Err(anyhow!("Destination is empty (0 bytes): {}", dest.display()));
        }

        Ok(())
    }

    fn verify_copy(&self, file: &BookFile, dest: &Path) -> Result<()> {
        let src_hash = hash_file(&file.path)?;
        let dst_hash = hash_file(dest)?;

        if src_hash != dst_hash {
            return Err(anyhow!(
                "Hash mismatch: {} != {}",
                src_hash,
                dst_hash
            ));
        }

        Ok(())
    }

    pub fn destination_path(&self, file: &BookFile) -> Result<PathBuf> {
        let default_metadata = crate::types::Metadata::default();
        let meta = file.metadata.as_ref().unwrap_or(&default_metadata);
        let author = meta.author.as_deref().unwrap_or("Unknown Author");
        let title = meta.title.as_deref().unwrap_or(&file.name);
        let ext = &file.ext;

        let clean_title = title
            .replace(".epub", "")
            .replace(".pdf", "")
            .replace(".mobi", "")
            .replace(".azw3", "")
            .replace(".m4b", "")
            .replace(".mp3", "");

        let filename = format!("{}.{}", clean_title, ext);

        let path = match self.structure {
            OrganizationStructure::AuthorTitle => {
                self.output_dir.join(author).join(&filename)
            }
            OrganizationStructure::AuthorSeriesTitle => {
                let series = meta.series.as_deref().unwrap_or("Unknown Series");
                self.output_dir.join(author).join(series).join(&filename)
            }
            OrganizationStructure::Title => {
                self.output_dir.join(&filename)
            }
        };

        Ok(path)
    }

    fn rename_duplicate(&self, path: &Path) -> Result<PathBuf> {
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let ext = path.extension().unwrap_or_default().to_string_lossy();
        let parent = path.parent().unwrap_or(Path::new(""));

        let mut counter = 1;
        let mut new_path = path.to_path_buf();

        while new_path.exists() {
            new_path = parent.join(format!("{}_{}.{}", stem, counter, ext));
            counter += 1;
        }

        Ok(new_path)
    }
}
