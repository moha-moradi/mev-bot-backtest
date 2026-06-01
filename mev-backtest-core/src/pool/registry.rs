use std::path::Path;

use crate::pool::state::PoolInfo;

/// Loads pool lists from JSON files on disk.
pub struct PoolRegistry;

impl PoolRegistry {
    /// Load pool info from a JSON file.
    /// Expected format: array of PoolInfo objects.
    pub fn load(path: &str) -> anyhow::Result<Vec<PoolInfo>> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read pool registry '{}': {}", path, e))?;
        let pools: Vec<PoolInfo> = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse pool registry '{}': {}", path, e))?;
        Ok(pools)
    }

    /// Load pool info from an optional path; returns empty vec if path is None or missing.
    pub fn load_optional(path: Option<&str>) -> Vec<PoolInfo> {
        match path {
            Some(p) if Path::new(p).exists() => Self::load(p).unwrap_or_default(),
            _ => Vec::new(),
        }
    }
}
