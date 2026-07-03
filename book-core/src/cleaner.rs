use crate::types::{BookFile};
use crate::config::MappingsConfig;
use regex::Regex;

pub struct Cleaner {
    pub remove_brackets: bool,
    pub remove_parentheses: bool,
    pub normalize_underscores: bool,
    pub normalize_spaces: bool,
    pub mappings: MappingsConfig,
}

impl Default for Cleaner {
    fn default() -> Self {
        Self::new()
    }
}

impl Cleaner {
    pub fn new() -> Self {
        Self {
            remove_brackets: true,
            remove_parentheses: true,
            normalize_underscores: true,
            normalize_spaces: true,
            mappings: MappingsConfig::default(),
        }
    }

    pub fn with_mappings(mappings: MappingsConfig) -> Self {
        Self {
            remove_brackets: true,
            remove_parentheses: true,
            normalize_underscores: true,
            normalize_spaces: true,
            mappings,
        }
    }

    pub fn clean_filename(&self, name: &str) -> String {
        let mut result = name.to_string();

        if self.remove_brackets {
            let bracket_re = Regex::new(r"\[[^\]]*\]").unwrap();
            result = bracket_re.replace_all(&result, "").to_string();
        }

        if self.remove_parentheses {
            let paren_re = Regex::new(r"\([^)]*\)").unwrap();
            result = paren_re.replace_all(&result, "").to_string();
        }

        if self.normalize_underscores {
            result = result.replace('_', " - ");
        }

        if self.normalize_spaces {
            let space_re = Regex::new(r"\s+").unwrap();
            result = space_re.replace_all(&result, " ").to_string();
            result = result.trim().to_string();
        }

        result
    }

    pub fn extract_metadata(&self, file: &BookFile) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
        let clean = self.clean_filename(&file.name);
        let name = clean.trim();
        let ext = &file.ext;

        let name_without_ext = name.trim_end_matches(&format!(".{}", ext)).trim();

        if let Some((author, title, narrator)) = self.extract_author_title_narrator(name_without_ext) {
            return (Some(author), Some(title), Some(narrator), None);
        }

        if let Some((author, title)) = self.extract_author_title(name_without_ext) {
            return (Some(author), Some(title), None, None);
        }

        if let Some((title, author)) = self.extract_title_author(name_without_ext) {
            return (Some(author), Some(title), None, None);
        }

        if let Some((author, series, title)) = self.extract_series(name_without_ext) {
            return (Some(author), Some(title), None, Some(series));
        }

        if let Some((title, author)) = self.extract_title_with_author(name_without_ext) {
            return (Some(author), Some(title), None, None);
        }

        (None, Some(name_without_ext.to_string()), None, None)
    }

    fn extract_author_title_narrator(&self, name: &str) -> Option<(String, String, String)> {
        // Try to parse "Author - Title_Narrator" pattern
        let parts: Vec<&str> = name.split(" - ").collect();
        if parts.len() >= 2 {
            let author = parts[0].trim();
            let rest = parts[1..].join(" - ");
            
            // Check if rest ends with a known narrator
            for narrator in &self.mappings.narrators {
                if rest.ends_with(narrator) {
                    let title = rest[..rest.len() - narrator.len()].trim_end_matches(|c| c == ' ' || c == '_' || c == '-');
                    return Some((author.to_string(), title.to_string(), narrator.to_string()));
                }
            }
        }
        None
    }

    fn extract_author_title(&self, name: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = name.split(" - ").collect();
        if parts.len() >= 2 {
            let first = parts[0].trim();
            if self.is_author(first) {
                let rest = parts[1..].join(" - ");
                return Some((first.to_string(), rest));
            }
        }
        None
    }

    fn extract_title_author(&self, name: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = name.split(" - ").collect();
        if parts.len() >= 2 {
            let last_part = parts.last().unwrap_or(&"").trim();
            if self.is_author(last_part) {
                let title = parts[..parts.len()-1].join(" - ");
                return Some((title, last_part.to_string()));
            }
        }
        None
    }

    fn extract_series(&self, name: &str) -> Option<(String, String, String)> {
        let parts: Vec<&str> = name.split(" - ").collect();
        if parts.len() >= 3 {
            let author = parts[0].trim();
            let series = parts[1].trim();
            let title = parts[2..].join(" - ");
            
            if self.is_author(author) && series.chars().any(|c| c.is_ascii_digit()) {
                return Some((author.to_string(), series.to_string(), title));
            }
        }
        None
    }

    fn extract_title_with_author(&self, name: &str) -> Option<(String, String)> {
        let re1 = Regex::new(r"^(.+?)\s*\(([^)]+)\)$").unwrap();
        let re2 = Regex::new(r"^(.+?)\s*\[([^\]]+)\]$").unwrap();

        if let Some(caps) = re1.captures(name) {
            let title = caps[1].trim().to_string();
            let author = caps[2].trim().to_string();
            if self.is_author(&author) {
                return Some((title, author));
            }
        }

        if let Some(caps) = re2.captures(name) {
            let title = caps[1].trim().to_string();
            let author = caps[2].trim().to_string();
            if self.is_author(&author) {
                return Some((title, author));
            }
        }

        None
    }

    fn is_author(&self, name: &str) -> bool {
        if name.contains(',') {
            return true;
        }

        for author in &self.mappings.authors {
            if name.contains(author) {
                return true;
            }
        }

        for narrator in &self.mappings.narrators {
            if name == *narrator {
                return false;
            }
        }

        if name.starts_with("The ") || name.starts_with("A ") || name.starts_with("An ") {
            return false;
        }

        if name.len() < 25 {
            return true;
        }

        false
    }
}
