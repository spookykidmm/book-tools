use crate::hashing::hash_file;
use crate::types::BookFile;
use crate::registry::BookRecord;
use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    Process,
    Skip,
    Reprocess,
    Quarantine,
    Duplicate { of_hash: String },
}

#[derive(Debug, Clone)]
pub struct DecisionContext {
    pub file: BookFile,
    pub registry_record: Option<BookRecord>,
    pub run_seen_hashes: HashSet<String>,
    pub output_exists: bool,
}

#[derive(Debug, Clone)]
pub enum DedupePolicy {
    KeepLargest,
    KeepBestBitrate,
    KeepFirst,
    KeepAll,
}

pub struct DecisionEngine {
    dedupe_policy: DedupePolicy,
}

impl DecisionEngine {
    pub fn new(policy: DedupePolicy) -> Self {
        Self { dedupe_policy: policy }
    }

    pub fn decide(&self, ctx: &DecisionContext) -> Result<Decision> {
        if ctx.file.size == 0 {
            return Ok(Decision::Quarantine);
        }

        let hash = match &ctx.file.content_hash {
            Some(h) => h,
            None => return Ok(Decision::Quarantine),
        };

        if ctx.registry_record.is_some() {
            if ctx.output_exists {
                return Ok(Decision::Skip);
            } else {
                return Ok(Decision::Reprocess);
            }
        }

        if ctx.run_seen_hashes.contains(hash) {
            return Ok(Decision::Duplicate {
                of_hash: hash.clone(),
            });
        }

        Ok(Decision::Process)
    }

    pub fn evaluate_duplicate_group(
        &self,
        mut records: Vec<BookRecord>,
    ) -> Result<Vec<BookRecord>> {
        if records.len() <= 1 {
            return Ok(records);
        }

        match self.dedupe_policy {
            DedupePolicy::KeepLargest | DedupePolicy::KeepBestBitrate => {
                records.sort_by(|a, b| b.size.cmp(&a.size));
                Ok(vec![records.remove(0)])
            }
            DedupePolicy::KeepFirst => {
                records.sort_by(|a, b| a.last_seen.cmp(&b.last_seen));
                Ok(vec![records.remove(0)])
            }
            DedupePolicy::KeepAll => Ok(records),
        }
    }

    pub fn verify_file_integrity(&self, expected_hash: &str, file_path: &str) -> Result<bool> {
        let path = Path::new(file_path);
        let actual_hash = hash_file(path)?;
        Ok(actual_hash == expected_hash)
    }
}
