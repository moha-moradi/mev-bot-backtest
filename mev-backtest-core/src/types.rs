use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ChainName {
    Polygon,
    Avalanche,
    Bsc,
    Arbitrum,
    Base,
    Ethereum,
    Optimism,
}

impl ChainName {
    pub fn chain_id(self) -> u64 {
        match self {
            ChainName::Polygon => 137,
            ChainName::Avalanche => 43114,
            ChainName::Bsc => 56,
            ChainName::Arbitrum => 42161,
            ChainName::Base => 8453,
            ChainName::Ethereum => 1,
            ChainName::Optimism => 10,
        }
    }

    /// Public (free-tier) RPC endpoint — no API key required.
    pub fn public_rpc_url(&self) -> &'static str {
        match self {
            ChainName::Polygon => "https://polygon-bor.publicnode.com",
            ChainName::Avalanche => "https://avalanche-c-chain.publicnode.com",
            ChainName::Bsc => "https://bsc.publicnode.com",
            ChainName::Arbitrum => "https://arbitrum-one.publicnode.com",
            ChainName::Base => "https://base.publicnode.com",
            ChainName::Ethereum => "https://ethereum-rpc.publicnode.com",
            ChainName::Optimism => "https://optimism-rpc.publicnode.com",
        }
    }

    pub fn all() -> &'static [ChainName] {
        &[
            ChainName::Polygon,
            ChainName::Avalanche,
            ChainName::Bsc,
            ChainName::Arbitrum,
            ChainName::Base,
            ChainName::Ethereum,
            ChainName::Optimism,
        ]
    }
}

impl fmt::Display for ChainName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChainName::Polygon => write!(f, "polygon"),
            ChainName::Avalanche => write!(f, "avalanche"),
            ChainName::Bsc => write!(f, "bsc"),
            ChainName::Arbitrum => write!(f, "arbitrum"),
            ChainName::Base => write!(f, "base"),
            ChainName::Ethereum => write!(f, "ethereum"),
            ChainName::Optimism => write!(f, "optimism"),
        }
    }
}

impl FromStr for ChainName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "polygon" => Ok(ChainName::Polygon),
            "avalanche" => Ok(ChainName::Avalanche),
            "bsc" => Ok(ChainName::Bsc),
            "arbitrum" => Ok(ChainName::Arbitrum),
            "base" => Ok(ChainName::Base),
            "ethereum" => Ok(ChainName::Ethereum),
            "optimism" => Ok(ChainName::Optimism),
            _ => Err(format!(
                "unknown chain '{s}'. Supported: {}",
                ChainName::all()
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FlashLoanProvider {
    Auto,
    Balancer,
    Aave,
    Uniswap,
}

impl fmt::Display for FlashLoanProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlashLoanProvider::Auto => write!(f, "auto"),
            FlashLoanProvider::Balancer => write!(f, "balancer"),
            FlashLoanProvider::Aave => write!(f, "aave"),
            FlashLoanProvider::Uniswap => write!(f, "uniswap"),
        }
    }
}

impl FromStr for FlashLoanProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(FlashLoanProvider::Auto),
            "balancer" => Ok(FlashLoanProvider::Balancer),
            "aave" => Ok(FlashLoanProvider::Aave),
            "uniswap" => Ok(FlashLoanProvider::Uniswap),
            _ => Err(format!(
                "unknown flash loan provider '{s}'. Supported: auto, balancer, aave, uniswap"
            )),
        }
    }
}

impl FlashLoanProvider {
    pub fn is_forced(self) -> bool {
        self != FlashLoanProvider::Auto
    }

    pub fn priority_list(auto_mode: bool) -> &'static [FlashLoanProvider] {
        if auto_mode {
            &[
                FlashLoanProvider::Balancer,
                FlashLoanProvider::Aave,
                FlashLoanProvider::Uniswap,
            ]
        } else {
            &[]
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Strategy {
    TwoHopArb,
}

impl fmt::Display for Strategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Strategy::TwoHopArb => write!(f, "two_hop_arb"),
        }
    }
}

impl FromStr for Strategy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "two_hop_arb" => Ok(Strategy::TwoHopArb),
            _ => Err(format!(
                "unknown strategy '{s}'. Supported: two_hop_arb, all"
            )),
        }
    }
}

impl Strategy {
    pub fn all() -> &'static [Strategy] {
        &[Strategy::TwoHopArb]
    }

    pub fn from_comma_list(s: &str) -> Result<Vec<Strategy>, String> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("all") {
            return Ok(Strategy::all().to_vec());
        }
        s.split(',')
            .map(|part| part.trim().parse::<Strategy>())
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeMode {
    Days(u64),
    Blocks(u64),
    Single(u64),
    Range(u64, u64),
}

impl fmt::Display for RangeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RangeMode::Days(n) => write!(f, "last {n} days"),
            RangeMode::Blocks(n) => write!(f, "last {n} blocks"),
            RangeMode::Single(n) => write!(f, "single block #{n}"),
            RangeMode::Range(a, b) => write!(f, "blocks {a}–{b} ({} blocks)", b - a + 1),
        }
    }
}

impl RangeMode {
    pub fn resolve_description(&self) -> String {
        match self {
            RangeMode::Days(_) => "resolves at runtime via binary search on timestamps".to_string(),
            RangeMode::Blocks(_) => "resolves at runtime from chain tip".to_string(),
            RangeMode::Single(_) => "single block mode".to_string(),
            RangeMode::Range(from, to) => format!("blocks {from}–{to} ({} blocks)", to - from + 1),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GasModel {
    #[serde(rename = "historical_exact")]
    HistoricalExact,
    #[serde(rename = "p90")]
    P90,
    #[serde(rename = "fixed")]
    Fixed,
}

impl fmt::Display for GasModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GasModel::HistoricalExact => write!(f, "historical_exact"),
            GasModel::P90 => write!(f, "p90"),
            GasModel::Fixed => write!(f, "fixed"),
        }
    }
}

impl FromStr for GasModel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "historical_exact" => Ok(GasModel::HistoricalExact),
            "p90" => Ok(GasModel::P90),
            "fixed" => Ok(GasModel::Fixed),
            _ => Err(format!(
                "unknown gas model '{s}'. Supported: historical_exact, p90, fixed"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OutputFormat {
    #[serde(rename = "table")]
    Table,
    #[serde(rename = "csv")]
    Csv,
    #[serde(rename = "json")]
    Json,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputFormat::Table => write!(f, "table"),
            OutputFormat::Csv => write!(f, "csv"),
            OutputFormat::Json => write!(f, "json"),
        }
    }
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" => Ok(OutputFormat::Table),
            "csv" => Ok(OutputFormat::Csv),
            "json" => Ok(OutputFormat::Json),
            _ => Err(format!(
                "unknown output format '{s}'. Supported: table, csv, json"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_name_roundtrip() {
        for chain in ChainName::all() {
            let s = chain.to_string();
            let parsed: ChainName = s.parse().unwrap();
            assert_eq!(*chain, parsed);
        }
    }

    #[test]
    fn test_chain_name_unknown() {
        let err = "unknown".parse::<ChainName>().unwrap_err();
        assert!(err.contains("unknown chain"));
    }

    #[test]
    fn test_chain_name_chain_id() {
        assert_eq!(ChainName::Polygon.chain_id(), 137);
        assert_eq!(ChainName::Ethereum.chain_id(), 1);
    }

    #[test]
    fn test_flash_loan_roundtrip() {
        for (p, s) in &[
            (FlashLoanProvider::Auto, "auto"),
            (FlashLoanProvider::Balancer, "balancer"),
            (FlashLoanProvider::Aave, "aave"),
            (FlashLoanProvider::Uniswap, "uniswap"),
        ] {
            assert_eq!(p.to_string(), *s);
            assert_eq!(s.parse::<FlashLoanProvider>().unwrap(), *p);
        }
    }

    #[test]
    fn test_flash_loan_is_forced() {
        assert!(!FlashLoanProvider::Auto.is_forced());
        assert!(FlashLoanProvider::Balancer.is_forced());
        assert!(FlashLoanProvider::Aave.is_forced());
        assert!(FlashLoanProvider::Uniswap.is_forced());
    }

    #[test]
    fn test_flash_loan_priority_list() {
        assert_eq!(FlashLoanProvider::priority_list(true).len(), 3);
        assert!(FlashLoanProvider::priority_list(false).is_empty());
    }

    #[test]
    fn test_strategy_roundtrip() {
        assert_eq!(Strategy::TwoHopArb.to_string(), "two_hop_arb");
        assert_eq!("two_hop_arb".parse::<Strategy>().unwrap(), Strategy::TwoHopArb);
    }

    #[test]
    fn test_strategy_unknown() {
        assert!("sandwich".parse::<Strategy>().unwrap_err().contains("unknown strategy"));
    }

    #[test]
    fn test_strategy_from_comma_list_single() {
        let v = Strategy::from_comma_list("two_hop_arb").unwrap();
        assert_eq!(v, vec![Strategy::TwoHopArb]);
    }

    #[test]
    fn test_strategy_from_comma_list_all() {
        let v = Strategy::from_comma_list("all").unwrap();
        assert_eq!(v, Strategy::all());
    }

    #[test]
    fn test_strategy_all_static() {
        assert_eq!(Strategy::all(), &[Strategy::TwoHopArb]);
    }

    #[test]
    fn test_range_mode_display() {
        assert_eq!(RangeMode::Days(7).to_string(), "last 7 days");
        assert_eq!(RangeMode::Blocks(100).to_string(), "last 100 blocks");
        assert_eq!(RangeMode::Single(42).to_string(), "single block #42");
        assert_eq!(RangeMode::Range(10, 20).to_string(), "blocks 10–20 (11 blocks)");
    }

    #[test]
    fn test_range_mode_resolve_description() {
        assert!(RangeMode::Days(1).resolve_description().contains("binary search"));
        assert!(RangeMode::Blocks(1).resolve_description().contains("chain tip"));
        assert_eq!(RangeMode::Single(5).resolve_description(), "single block mode");
        assert!(RangeMode::Range(1, 10).resolve_description().contains("blocks 1–10"));
    }

    #[test]
    fn test_gas_model_roundtrip() {
        for m in &[GasModel::HistoricalExact, GasModel::P90, GasModel::Fixed] {
            let s = m.to_string();
            assert_eq!(s.parse::<GasModel>().unwrap(), *m);
        }
    }

    #[test]
    fn test_gas_model_unknown() {
        assert!("foo".parse::<GasModel>().unwrap_err().contains("unknown gas model"));
    }

    #[test]
    fn test_output_format_roundtrip() {
        for f in &[OutputFormat::Table, OutputFormat::Csv, OutputFormat::Json] {
            let s = f.to_string();
            assert_eq!(s.parse::<OutputFormat>().unwrap(), *f);
        }
    }

    #[test]
    fn test_output_format_unknown() {
        assert!("xml".parse::<OutputFormat>().unwrap_err().contains("unknown output format"));
    }
}


