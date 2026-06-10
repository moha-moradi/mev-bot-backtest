use axum::Json;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DexConfig {
    pub id: String,
    pub name: String,
    pub fork: String,
    pub router: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChainConfigResponse {
    pub id: String,
    pub name: String,
    pub native_token: String,
    pub color: String,
    pub block_time: f64,
    pub rpc_default: String,
    pub explorer_base: String,
    pub dexes: Vec<DexConfig>,
    pub flash_loan_providers: Vec<String>,
    pub coingecko_id: String,
    pub activity_multiplier: f64,
    pub avg_tx_per_block: f64,
    pub gas_price_gwei: f64,
    pub native_usd: f64,
}

fn ethereum_dexes() -> Vec<DexConfig> {
    vec![
        DexConfig { id: "uni-v2".into(), name: "Uniswap v2".into(), fork: "UniV2".into(), router: "0x7a25...488D".into() },
        DexConfig { id: "uni-v3".into(), name: "Uniswap v3".into(), fork: "UniV3".into(), router: "0xE592...5A67".into() },
        DexConfig { id: "sushi".into(), name: "SushiSwap".into(), fork: "UniV2".into(), router: "0xd9e1...78B9".into() },
        DexConfig { id: "curve".into(), name: "Curve".into(), fork: "Curve".into(), router: "0xDCEe...0bb6".into() },
    ]
}

fn polygon_dexes() -> Vec<DexConfig> {
    vec![
        DexConfig { id: "uni-v2".into(), name: "QuickSwap".into(), fork: "UniV2".into(), router: "0xa5E0...e38B".into() },
        DexConfig { id: "uni-v3".into(), name: "Uniswap v3".into(), fork: "UniV3".into(), router: "0xE592...5A67".into() },
        DexConfig { id: "sushi".into(), name: "SushiSwap".into(), fork: "UniV2".into(), router: "0x1b02...8Ab2".into() },
        DexConfig { id: "curve".into(), name: "Curve".into(), fork: "Curve".into(), router: "0x7De0...2b3b".into() },
    ]
}

fn bsc_dexes() -> Vec<DexConfig> {
    vec![
        DexConfig { id: "uni-v2".into(), name: "PancakeSwap v2".into(), fork: "UniV2".into(), router: "0x10ED...5733".into() },
        DexConfig { id: "uni-v3".into(), name: "PancakeSwap v3".into(), fork: "UniV3".into(), router: "0x1b3d...28D7".into() },
        DexConfig { id: "sushi".into(), name: "SushiSwap".into(), fork: "UniV2".into(), router: "0x1b02...8Ab2".into() },
    ]
}

fn arbitrum_dexes() -> Vec<DexConfig> {
    vec![
        DexConfig { id: "uni-v2".into(), name: "Camelot".into(), fork: "UniV2".into(), router: "0xc873...f4Fc".into() },
        DexConfig { id: "uni-v3".into(), name: "Uniswap v3".into(), fork: "UniV3".into(), router: "0xE592...5A67".into() },
        DexConfig { id: "sushi".into(), name: "SushiSwap".into(), fork: "UniV2".into(), router: "0x1b02...8Ab2".into() },
        DexConfig { id: "curve".into(), name: "Curve".into(), fork: "Curve".into(), router: "0x10Fe...61d5".into() },
    ]
}

fn avalanche_dexes() -> Vec<DexConfig> {
    vec![
        DexConfig { id: "uni-v2".into(), name: "Trader Joe".into(), fork: "UniV2".into(), router: "0x60aE...a62c".into() },
        DexConfig { id: "uni-v3".into(), name: "Uniswap v3".into(), fork: "UniV3".into(), router: "0xE592...5A67".into() },
        DexConfig { id: "sushi".into(), name: "SushiSwap".into(), fork: "UniV2".into(), router: "0x1b02...8Ab2".into() },
    ]
}

fn base_dexes() -> Vec<DexConfig> {
    vec![
        DexConfig { id: "uni-v3".into(), name: "Aerodrome".into(), fork: "Solidly".into(), router: "0xcF77...8046".into() },
        DexConfig { id: "uni-v2".into(), name: "Uniswap v2".into(), fork: "UniV2".into(), router: "0x7a25...488D".into() },
    ]
}

fn optimism_dexes() -> Vec<DexConfig> {
    vec![
        DexConfig { id: "uni-v2".into(), name: "SushiSwap".into(), fork: "UniV2".into(), router: "0x1b02...8Ab2".into() },
        DexConfig { id: "uni-v3".into(), name: "Uniswap v3".into(), fork: "UniV3".into(), router: "0xE592...5A67".into() },
    ]
}

fn all_chains() -> Vec<ChainConfigResponse> {
    vec![
        ChainConfigResponse {
            id: "ethereum".into(),
            name: "Ethereum".into(),
            native_token: "ETH".into(),
            color: "#627EEA".into(),
            block_time: 12.0,
            rpc_default: "https://ethereum-rpc.publicnode.com".into(),
            explorer_base: "https://etherscan.io/tx/".into(),
            dexes: ethereum_dexes(),
            flash_loan_providers: vec!["Balancer v2".into(), "Aave v3".into(), "Uniswap Flash Swap".into()],
            coingecko_id: "ethereum".into(),
            activity_multiplier: 1.0,
            avg_tx_per_block: 160.0,
            gas_price_gwei: 25.0,
            native_usd: 3200.0,
        },
        ChainConfigResponse {
            id: "polygon".into(),
            name: "Polygon".into(),
            native_token: "MATIC".into(),
            color: "#8247E5".into(),
            block_time: 2.0,
            rpc_default: "https://polygon-bor.publicnode.com".into(),
            explorer_base: "https://polygonscan.com/tx/".into(),
            dexes: polygon_dexes(),
            flash_loan_providers: vec!["Balancer v2".into(), "Aave v3".into()],
            coingecko_id: "matic-network".into(),
            activity_multiplier: 1.5,
            avg_tx_per_block: 400.0,
            gas_price_gwei: 50.0,
            native_usd: 0.85,
        },
        ChainConfigResponse {
            id: "bsc".into(),
            name: "BSC".into(),
            native_token: "BNB".into(),
            color: "#F0B90B".into(),
            block_time: 3.0,
            rpc_default: "https://bsc.publicnode.com".into(),
            explorer_base: "https://bscscan.com/tx/".into(),
            dexes: bsc_dexes(),
            flash_loan_providers: vec!["Balancer v2".into(), "Aave v3".into()],
            coingecko_id: "binancecoin".into(),
            activity_multiplier: 2.0,
            avg_tx_per_block: 300.0,
            gas_price_gwei: 5.0,
            native_usd: 580.0,
        },
        ChainConfigResponse {
            id: "arbitrum".into(),
            name: "Arbitrum".into(),
            native_token: "ETH".into(),
            color: "#2D374B".into(),
            block_time: 0.25,
            rpc_default: "https://arbitrum-one.publicnode.com".into(),
            explorer_base: "https://arbiscan.io/tx/".into(),
            dexes: arbitrum_dexes(),
            flash_loan_providers: vec!["Balancer v2".into(), "Aave v3".into()],
            coingecko_id: "arbitrum".into(),
            activity_multiplier: 2.5,
            avg_tx_per_block: 600.0,
            gas_price_gwei: 0.1,
            native_usd: 3200.0,
        },
        ChainConfigResponse {
            id: "avalanche".into(),
            name: "Avalanche".into(),
            native_token: "AVAX".into(),
            color: "#E84142".into(),
            block_time: 2.0,
            rpc_default: "https://avalanche-c-chain.publicnode.com".into(),
            explorer_base: "https://snowtrace.io/tx/".into(),
            dexes: avalanche_dexes(),
            flash_loan_providers: vec!["Balancer v2".into(), "Aave v3".into()],
            coingecko_id: "avalanche-2".into(),
            activity_multiplier: 1.2,
            avg_tx_per_block: 200.0,
            gas_price_gwei: 25.0,
            native_usd: 35.0,
        },
        ChainConfigResponse {
            id: "base".into(),
            name: "Base".into(),
            native_token: "ETH".into(),
            color: "#0052FF".into(),
            block_time: 2.0,
            rpc_default: "https://base.publicnode.com".into(),
            explorer_base: "https://basescan.org/tx/".into(),
            dexes: base_dexes(),
            flash_loan_providers: vec!["Balancer v2".into(), "Aave v3".into()],
            coingecko_id: "base".into(),
            activity_multiplier: 1.8,
            avg_tx_per_block: 300.0,
            gas_price_gwei: 0.1,
            native_usd: 3200.0,
        },
        ChainConfigResponse {
            id: "optimism".into(),
            name: "Optimism".into(),
            native_token: "ETH".into(),
            color: "#FF0420".into(),
            block_time: 2.0,
            rpc_default: "https://optimism-rpc.publicnode.com".into(),
            explorer_base: "https://optimistic.etherscan.io/tx/".into(),
            dexes: optimism_dexes(),
            flash_loan_providers: vec!["Balancer v2".into(), "Aave v3".into()],
            coingecko_id: "optimism".into(),
            activity_multiplier: 1.3,
            avg_tx_per_block: 200.0,
            gas_price_gwei: 0.1,
            native_usd: 3200.0,
        },
    ]
}

pub async fn get_chains() -> Json<Vec<ChainConfigResponse>> {
    Json(all_chains())
}
