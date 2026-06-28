use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub paths: PathsConfig,
    pub cleaning: CleaningConfig,
    pub merging: MergingConfig,
    pub metadata: MetadataConfig,
    pub organization: OrganizationConfig,
    pub mappings: MappingsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PathsConfig {
    pub source: Option<PathBuf>,
    pub output: Option<PathBuf>,
    pub quarantine: Option<PathBuf>,
    pub logs: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleaningConfig {
    pub remove_brackets: bool,
    pub remove_parentheses: bool,
    pub normalize_underscores: bool,
    pub normalize_spaces: bool,
}

impl Default for CleaningConfig {
    fn default() -> Self {
        Self {
            remove_brackets: true,
            remove_parentheses: true,
            normalize_underscores: true,
            normalize_spaces: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergingConfig {
    pub enabled: bool,
    pub mode: String,
    pub bitrate: u32,
}

impl Default for MergingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: "copy".to_string(),
            bitrate: 128,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataConfig {
    pub extract_from_filename: bool,
    pub extract_from_folder: bool,
    pub extract_from_id3: bool,
}

impl Default for MetadataConfig {
    fn default() -> Self {
        Self {
            extract_from_filename: true,
            extract_from_folder: true,
            extract_from_id3: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationConfig {
    pub structure: String,
    pub handle_duplicates: String,
    pub move_files: bool,
}

impl Default for OrganizationConfig {
    fn default() -> Self {
        Self {
            structure: "author/title".to_string(),
            handle_duplicates: "rename".to_string(),
            move_files: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MappingsConfig {
    pub authors: Vec<String>,
    pub narrators: Vec<String>,
    pub audiobook_patterns: HashMap<String, AudiobookMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudiobookMapping {
    pub author: String,
    pub title: String,
}

impl Config {
    pub fn default_config() -> Self {
        Self {
            paths: PathsConfig::default(),
            cleaning: CleaningConfig::default(),
            merging: MergingConfig::default(),
            metadata: MetadataConfig::default(),
            organization: OrganizationConfig::default(),
            mappings: MappingsConfig::default(),
        }
    }

    pub fn from_file(path: &PathBuf) -> Result<Self, anyhow::Error> {
        let content = std::fs::read_to_string(path)?;
        let config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), anyhow::Error> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
