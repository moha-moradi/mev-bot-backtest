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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::dex_type::DexType;

    #[test]
    fn test_load_nonexistent_file() {
        let result = PoolRegistry::load("/nonexistent/path.json");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Failed to read pool registry"));
    }

    #[test]
    fn test_load_invalid_json() {
        let dir = std::env::temp_dir().join("pool_reg_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("invalid.json");
        std::fs::write(&path, "this is not json").unwrap();
        let result = PoolRegistry::load(path.to_str().unwrap());
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Failed to parse pool registry"));
    }

    #[test]
    fn test_load_optional_none() {
        let pools = PoolRegistry::load_optional(None);
        assert!(pools.is_empty());
    }

    #[test]
    fn test_load_optional_nonexistent() {
        let pools = PoolRegistry::load_optional(Some("/nonexistent/path.json"));
        assert!(pools.is_empty());
    }

    #[test]
    fn test_load_optional_valid_json() {
        let dir = std::env::temp_dir().join("pool_reg_test2");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("valid.json");
        let json = r#"[
            {
                "address": "0x6e7a5fafcec6bb1e78bae2a1f0b612012bf14827",
                "token0": "0x2791bca1f2de4661ed88a30c99a7a9449aa84174",
                "token1": "0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270",
                "name": "QuickSwap WMATIC/USDC",
                "fee": 30,
                "type": "uniswap_v2",
                "tick_spacing": null
            }
        ]"#;
        std::fs::write(&path, json).unwrap();
        let pools = PoolRegistry::load_optional(Some(path.to_str().unwrap()));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
        assert_eq!(pools.len(), 1);
        assert_eq!(pools[0].name.as_deref(), Some("QuickSwap WMATIC/USDC"));
        assert_eq!(pools[0].fee, 30);
        assert_eq!(pools[0].dex_type, DexType::UniswapV2);
    }

    #[test]
    fn test_load_optional_empty_json_array() {
        let dir = std::env::temp_dir().join("pool_reg_test3");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("empty.json");
        std::fs::write(&path, "[]").unwrap();
        let pools = PoolRegistry::load_optional(Some(path.to_str().unwrap()));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
        assert!(pools.is_empty());
    }
}
