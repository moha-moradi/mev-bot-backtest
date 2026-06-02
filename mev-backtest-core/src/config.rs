use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::types::{
    ChainName, FlashLoanProvider, RangeMode, Strategy,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub chain_id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balancer_vault: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aave_v3_pool: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uniswap_v3_factory: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pools_registry_path: Option<String>,
    /// Uniswap V2 factory addresses for on-chain pool discovery
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uniswap_v2_factories: Option<Vec<String>>,
    /// Block number to start pool discovery scan from (default: genesis)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool_discovery_start_block: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // Chain and connection
    #[serde(default = "default_chain")]
    pub chain: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rpc_url: Option<String>,

    // Flash loan
    #[serde(default = "default_flash_loan_provider")]
    pub flash_loan_provider: String,

    // Strategies
    #[serde(default = "default_strategies")]
    pub strategies: String,

    // Gas model
    #[serde(default = "default_gas_model")]
    pub gas_model: String,

    // Output
    #[serde(default = "default_output_format")]
    pub output: String,
    #[serde(default = "default_export_path")]
    pub export_path: String,
    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,

    // Block range (not serialized to TOML directly, handled via CLI merge)
    #[serde(skip)]
    pub days: Option<u64>,
    #[serde(skip)]
    pub blocks: Option<u64>,
    #[serde(skip)]
    pub block: Option<u64>,
    #[serde(skip)]
    pub from_block: Option<u64>,
    #[serde(skip)]
    pub to_block: Option<u64>,

    // Per-chain configuration
    #[serde(default)]
    pub chains: HashMap<String, ChainConfig>,

    // Config file path
    #[serde(skip)]
    pub config_path: Option<PathBuf>,
}

fn default_chain() -> String {
    "polygon".to_string()
}

fn default_flash_loan_provider() -> String {
    "auto".to_string()
}

fn default_strategies() -> String {
    "all".to_string()
}

fn default_gas_model() -> String {
    "historical_exact".to_string()
}

fn default_output_format() -> String {
    "table".to_string()
}

fn default_export_path() -> String {
    "./results".to_string()
}

fn default_cache_dir() -> String {
    "./cache".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            chain: default_chain(),
            rpc_url: None,
            flash_loan_provider: default_flash_loan_provider(),
            strategies: default_strategies(),
            gas_model: default_gas_model(),
            output: default_output_format(),
            export_path: default_export_path(),
            cache_dir: default_cache_dir(),
            days: None,
            blocks: None,
            block: None,
            from_block: None,
            to_block: None,
            chains: default_chains(),
            config_path: None,
        }
    }
}

fn default_chains() -> HashMap<String, ChainConfig> {
    let mut m = HashMap::new();
    let polygon_factories = vec![
        "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32".to_string(), // QuickSwap
        "0x9e5A52f57b3038F1B8EeE45F28b3C1960e1fC6b".to_string(), // SushiSwap
    ];
    m.insert(
        "polygon".to_string(),
        ChainConfig {
            chain_id: 137,
            balancer_vault: Some("0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string()),
            aave_v3_pool: Some("0x794a61358D6845594F94dc1DB02A252b5b4814aD".to_string()),
            uniswap_v3_factory: Some("0x1F98431c8aD98523631AE4a59f267346ea31F984".to_string()),
            pools_registry_path: Some("./pools/polygon.json".to_string()),
            uniswap_v2_factories: Some(polygon_factories),
            pool_discovery_start_block: None,
        },
    );
    let avalanche_factories = vec![
        "0x9e5A52f57b3038F1B8EeE45F28b3C1960e1fC6b".to_string(), // SushiSwap (Trader Joe uses different)
    ];
    m.insert(
        "avalanche".to_string(),
        ChainConfig {
            chain_id: 43114,
            balancer_vault: Some("0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string()),
            aave_v3_pool: Some("0x69FA688f1Dc47d4B5d8029D5a35FB7a548E0B9b0".to_string()),
            uniswap_v3_factory: Some("0x740bDAebB6F93dB927d3bc8E2fE5EDF4343B2925".to_string()),
            pools_registry_path: Some("./pools/avalanche.json".to_string()),
            uniswap_v2_factories: Some(avalanche_factories),
            pool_discovery_start_block: None,
        },
    );
    let bsc_factories = vec![
        "0xcA143Ce32Fe78f1f7019d7d551a6402fC5350c73".to_string(), // PancakeSwap V2
        "0x9e5A52f57b3038F1B8EeE45F28b3C1960e1fC6b".to_string(), // SushiSwap
    ];
    m.insert(
        "bsc".to_string(),
        ChainConfig {
            chain_id: 56,
            balancer_vault: Some("0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string()),
            aave_v3_pool: Some("0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2".to_string()),
            uniswap_v3_factory: Some("0xdB1d10011AD0Ff90774D0C6Bb92e5C5c8b4461F7".to_string()),
            pools_registry_path: Some("./pools/bsc.json".to_string()),
            uniswap_v2_factories: Some(bsc_factories),
            pool_discovery_start_block: None,
        },
    );
    m.insert(
        "arbitrum".to_string(),
        ChainConfig {
            chain_id: 42161,
            balancer_vault: Some("0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string()),
            aave_v3_pool: Some("0x794a61358D6845594F94dc1DB02A252b5b4814aD".to_string()),
            uniswap_v3_factory: Some("0x1F98431c8aD98523631AE4a59f267346ea31F984".to_string()),
            pools_registry_path: Some("./pools/arbitrum.json".to_string()),
            uniswap_v2_factories: None,
            pool_discovery_start_block: None,
        },
    );
    let base_factories = vec![
        "0x8909Dc15e40173Ff4699343b6eB8132c0eE88a14".to_string(), // Aerodrome
    ];
    m.insert(
        "base".to_string(),
        ChainConfig {
            chain_id: 8453,
            balancer_vault: Some("0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string()),
            aave_v3_pool: Some("0xA238Dd80C259a72e81d7e4664a9801593F98d1c5".to_string()),
            uniswap_v3_factory: Some("0x33128a8fC17869897dcE68Ed026d694621f6FDfD".to_string()),
            pools_registry_path: Some("./pools/base.json".to_string()),
            uniswap_v2_factories: Some(base_factories),
            pool_discovery_start_block: None,
        },
    );
    let ethereum_factories = vec![
        "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f".to_string(), // Uniswap V2
        "0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac".to_string(), // SushiSwap
        "0xB3e281E8c6c888A5BcBf1108E4aC13dA3F5B1c9".to_string(), // ShibaSwap
    ];
    m.insert(
        "ethereum".to_string(),
        ChainConfig {
            chain_id: 1,
            balancer_vault: Some("0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string()),
            aave_v3_pool: Some("0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2".to_string()),
            uniswap_v3_factory: Some("0x1F98431c8aD98523631AE4a59f267346ea31F984".to_string()),
            pools_registry_path: Some("./pools/ethereum.json".to_string()),
            uniswap_v2_factories: Some(ethereum_factories),
            pool_discovery_start_block: None,
        },
    );
    let optimism_factories = vec![
        "0x9e5A52f57b3038F1B8EeE45F28b3C1960e1fC6b".to_string(), // SushiSwap
    ];
    m.insert(
        "optimism".to_string(),
        ChainConfig {
            chain_id: 10,
            balancer_vault: Some("0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string()),
            aave_v3_pool: Some("0x794a61358D6845594F94dc1DB02A252b5b4814aD".to_string()),
            uniswap_v3_factory: Some("0x1F98431c8aD98523631AE4a59f267346ea31F984".to_string()),
            pools_registry_path: Some("./pools/optimism.json".to_string()),
            uniswap_v2_factories: Some(optimism_factories),
            pool_discovery_start_block: None,
        },
    );
    m
}

impl Config {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read config file '{}': {}", path, e))?;
        let mut cfg: Config = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config file '{}': {}", path, e))?;
        cfg.config_path = Some(PathBuf::from(path));
        Ok(cfg)
    }

    pub fn load_or_default(path: &str) -> Self {
        if let Ok(cfg) = Self::load(path) {
            cfg
        } else {
            Config::default()
        }
    }

    pub fn default_with_chains() -> Self {
        Config::default()
    }

    /// Resolved RPC URL: user-provided value, or the public fallback for the target chain.    
    pub fn effective_rpc_url(&self, chain: ChainName) -> String {
        self.rpc_url
            .clone()
            .unwrap_or_else(|| chain.public_rpc_url().to_string())
    }

    pub fn to_toml_string(&self) -> anyhow::Result<String> {
        let value = toml::Value::try_from(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;
        toml::to_string(&value)
            .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))
    }

    pub fn plan_summary(
        &self,
        chain_name: ChainName,
        chain_cfg: &ChainConfig,
        range_mode: &RangeMode,
        strategies: &[Strategy],
        provider: FlashLoanProvider,
    ) -> String {
        let provider_desc = match provider {
            FlashLoanProvider::Auto => "auto (Balancer V2 → Aave V3 → Uniswap Flash Swap)".to_string(),
            other => format!("forced ({other})"),
        };

        let strat_list = strategies
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            r#"Chain:           {} (chain ID {})
RPC:             {}
Block range:     {} → {}
Strategies:      {}
Flash loan:      {}
Gas model:       {}
Cache dir:       {}
"#,
            chain_name,
            chain_cfg.chain_id,
            &self.effective_rpc_url(chain_name),
            range_mode,
            range_mode.resolve_description(),
            strat_list,
            provider_desc,
            self.gas_model,
            self.cache_dir,
        )
    }
}

#[derive(Debug, Clone)]
pub struct CliOverrides {
    pub days: Option<u64>,
    pub blocks: Option<u64>,
    pub block: Option<u64>,
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
    pub chain: Option<String>,
    pub rpc_url: Option<String>,
    pub flash_loan_provider: Option<String>,
    pub strategies: Option<String>,
    pub gas_model: Option<String>,
    pub output: Option<String>,
    pub export_path: Option<String>,
    pub cache_dir: Option<String>,
}

impl Config {
    pub fn merge_cli(&mut self, overrides: &CliOverrides) {
        if let Some(v) = &overrides.days {
            self.days = Some(*v);
        }
        if let Some(v) = &overrides.blocks {
            self.blocks = Some(*v);
        }
        if let Some(v) = &overrides.block {
            self.block = Some(*v);
        }
        if let Some(v) = &overrides.from_block {
            self.from_block = Some(*v);
        }
        if let Some(v) = &overrides.to_block {
            self.to_block = Some(*v);
        }
        if let Some(v) = &overrides.chain {
            self.chain = v.clone();
        }
        if let Some(v) = &overrides.rpc_url {
            self.rpc_url = Some(v.clone());
        }
        if let Some(v) = &overrides.flash_loan_provider {
            self.flash_loan_provider = v.clone();
        }
        if let Some(v) = &overrides.strategies {
            self.strategies = v.clone();
        }
        if let Some(v) = &overrides.gas_model {
            self.gas_model = v.clone();
        }
        if let Some(v) = &overrides.output {
            self.output = v.clone();
        }
        if let Some(v) = &overrides.export_path {
            self.export_path = v.clone();
        }
        if let Some(v) = &overrides.cache_dir {
            self.cache_dir = v.clone();
        }
    }
}
