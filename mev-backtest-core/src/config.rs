use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RangeMode {
    Days(u64),
    Blocks(u64),
    Block(u64),
    FromTo { from_block: u64, to_block: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum FlashLoanProvider {
    #[default]
    Auto,
    BalancerV2,
    AaveV3,
    UniswapSwap,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ChainName {
    #[default]
    #[serde(rename = "ethereum")]
    Ethereum,
    #[serde(rename = "polygon")]
    Polygon,
    #[serde(rename = "arbitrum")]
    Arbitrum,
    #[serde(rename = "optimism")]
    Optimism,
}

impl std::fmt::Display for ChainName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainName::Ethereum => write!(f, "ethereum"),
            ChainName::Polygon => write!(f, "polygon"),
            ChainName::Arbitrum => write!(f, "arbitrum"),
            ChainName::Optimism => write!(f, "optimism"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChainConfig {
    pub rpc_url: Option<String>,
    pub chain_id: Option<u64>,
    pub balancer_vault: Option<String>,
    pub aave_pool: Option<String>,
    pub uniswap_router: Option<String>,
    pub weth: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GasModel {
    pub bribe_pct: f64,
    pub priority_fee: u64,
    pub parallelism: u64,
}

impl Default for GasModel {
    fn default() -> Self {
        GasModel {
            bribe_pct: 0.0,
            priority_fee: 1_000_000,
            parallelism: 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutputConfig {
    pub csv: bool,
    pub json: bool,
    pub min_profit_usd: f64,
}

impl Default for OutputConfig {
    fn default() -> Self {
        OutputConfig {
            csv: false,
            json: false,
            min_profit_usd: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub chain: ChainName,
    pub rpc_url: Option<String>,
    pub flash_loan_provider: FlashLoanProvider,
    pub strategies: Vec<String>,
    pub range_mode: Option<RangeMode>,
    pub gas_model: GasModel,
    pub output: OutputConfig,
    pub chains: std::collections::HashMap<String, ChainConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            chain: ChainName::Ethereum,
            rpc_url: None,
            flash_loan_provider: FlashLoanProvider::Auto,
            strategies: vec!["two_hop_arb".to_string(), "multi_hop_arb".to_string()],
            range_mode: None,
            gas_model: GasModel::default(),
            output: OutputConfig::default(),
            chains: [
                (
                    "ethereum".to_string(),
                    ChainConfig {
                        rpc_url: None,
                        chain_id: Some(1),
                        balancer_vault: Some("0xBA1222692F1d6b8fa77E1E8C4b3b8f5C7D0e5b3a".to_string()),
                        aave_pool: Some("0x8787f765cB5a9D4Ed99aB9b2e2c6F0F9B3c1d9cE".to_string()),
                        uniswap_router: Some("0xE592427A0AEce92D671a9B6C8e11B8a21c1f1aB1".to_string()),
                        weth: Some("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string()),
                    },
                ),
                (
                    "polygon".to_string(),
                    ChainConfig {
                        rpc_url: None,
                        chain_id: Some(137),
                        balancer_vault: Some("0xBA1222692F1d6b8fa77E1E8C4b3b8f5C7D0e5b3a".to_string()),
                        aave_pool: Some("0x794a61e9E7F2F7E9C8c6d4c6b3a8b2D9cE5f1aB3".to_string()),
                        uniswap_router: Some("0xE592427A0AEce92D671a9B6C8e11B8a21c1f1aB2".to_string()),
                        weth: Some("0x7ceB23fD6bC0adD59E62ac279EbFd0ace73699b9".to_string()),
                    },
                ),
                (
                    "arbitrum".to_string(),
                    ChainConfig {
                        rpc_url: None,
                        chain_id: Some(42161),
                        balancer_vault: Some("0xBA1222692F1d6b8fa77E1E8C4b3b8f5C7D0e5b3a".to_string()),
                        aave_pool: Some("0x794a61e9E7F2F7E9C8c6d4c6b3a8b2D9cE5f1aB4".to_string()),
                        uniswap_router: Some("0xE592427A0AEce92D671a9B6C8e11B8a21c1f1aB3".to_string()),
                        weth: Some("0x82af49447d8a79162f63f5aF9C495c3C2b9C4e1d".to_string()),
                    },
                ),
                (
                    "optimism".to_string(),
                    ChainConfig {
                        rpc_url: None,
                        chain_id: Some(10),
                        balancer_vault: Some("0xBA1222692F1d6b8fa77E1E8C4b3b8f5C7D0e5b3a".to_string()),
                        aave_pool: Some("0x794a61e9E7F2F7E9C8c6d4c6b3a8b2D9cE5f1aB5".to_string()),
                        uniswap_router: Some("0xE592427A0AEce92D671a9B6C8e11B8a21c1f1aB4".to_string()),
                        weth: Some("0x4200000000000000000000000000000000000006".to_string()),
                    },
                ),
            ].into_iter().collect(),
        }
    }
}

impl Config {
    pub fn load_toml(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn to_toml(&self) -> anyhow::Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        match &self.range_mode {
            Some(RangeMode::FromTo { from_block, to_block }) => {
                if from_block > to_block {
                    anyhow::bail!("--from-block ({}) cannot be greater than --to-block ({})", from_block, to_block);
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn get_chain_config(&self) -> anyhow::Result<&ChainConfig> {
        self.chains
            .get(&self.chain.to_string().to_lowercase())
            .ok_or_else(|| anyhow::anyhow!("Unknown chain: {}", self.chain))
    }
}

impl std::str::FromStr for ChainName {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        match s.to_lowercase().as_str() {
            "ethereum" => Ok(ChainName::Ethereum),
            "polygon" => Ok(ChainName::Polygon),
            "arbitrum" => Ok(ChainName::Arbitrum),
            "optimism" => Ok(ChainName::Optimism),
            _ => anyhow::bail!(
                "Unknown chain '{}'. Supported chains: ethereum, polygon, arbitrum, optimism",
                s
            ),
        }
    }
}

impl std::str::FromStr for FlashLoanProvider {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(FlashLoanProvider::Auto),
            "balancer" | "balancer_v2" => Ok(FlashLoanProvider::BalancerV2),
            "aave" | "aave_v3" => Ok(FlashLoanProvider::AaveV3),
            "uniswap" | "uniswap_swap" => Ok(FlashLoanProvider::UniswapSwap),
            _ => anyhow::bail!(
                "Unknown flash loan provider '{}'. Supported providers: auto, balancer_v2, aave_v3, uniswap_swap",
                s
            ),
        }
    }
}

pub fn supported_chains() -> &'static [&'static str] {
    &["ethereum", "polygon", "arbitrum", "optimism"]
}

pub fn supported_providers() -> &'static [&'static str] {
    &["auto", "balancer_v2", "aave_v3", "uniswap_swap"]
}

pub fn supported_strategies() -> &'static [&'static str] {
    &["two_hop_arb", "multi_hop_arb", "jit", "jit_arb", "sandwich"]
}