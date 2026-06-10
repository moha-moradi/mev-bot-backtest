/// Data contract for detected MEV opportunities and persisted results files.
///
/// These types are the serialization boundary between the core backtest engine,
/// the CLI output layer, and the API serialization layer.
use alloy::primitives::{Address, U256};
use serde::{Deserialize, Serialize};
use crate::types::Strategy;

/// A detected MEV opportunity from backtesting.
///
/// Different strategies populate different optional fields:
/// - `path` for multi-hop strategies,
/// - `tick_lower`/`tick_upper`/`liquidity_amount` for JIT strategies,
/// - `victim_tx_index`/`backrun_tx_index` for sandwich attacks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevOpportunity {
    /// Block where the opportunity was detected
    pub block_number: u64,
    /// Index of the transaction after which the opportunity exists
    pub tx_index: usize,
    /// The strategy type
    pub strategy: Strategy,
    /// Pool involved in the first swap
    pub pool_a: Address,
    /// Pool involved in the second swap
    pub pool_b: Address,
    /// Token being arbitraged (input token)
    pub token_in: Address,
    /// Token received as output
    pub token_out: Address,
    /// Amount of token_in to invest
    pub input_amount: U256,
    /// Expected profit in token_out (gross, before gas)
    pub expected_profit: U256,
    /// Estimated gas cost in wei
    pub gas_cost_wei: u128,
    /// Timestamp of the block
    pub timestamp: u64,
    /// Full pool path for multi-hop opportunities (e.g., [buy, intermediate, ..., sell])
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<Vec<Address>>,
    /// Tick range lower bound (JIT liquidity positions)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_lower: Option<i32>,
    /// Tick range upper bound (JIT liquidity positions)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_upper: Option<i32>,
    /// Amount of liquidity deployed (JIT positions)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub liquidity_amount: Option<u128>,
    /// Transaction index of the victim's swap (sandwich attacks)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub victim_tx_index: Option<usize>,
    /// Transaction index of the backrun (sandwich attacks)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backrun_tx_index: Option<usize>,
}

/// Saved results file wrapping opportunities with run metadata.
///
/// Written to `export_path` and re-read by the `report` subcommand.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultsFile {
    pub run_id: String,
    pub chain: String,
    pub start_block: u64,
    pub end_block: u64,
    pub range_mode: String,
    pub strategies: Vec<String>,
    pub flash_loan_provider: String,
    pub resolved_at: u64,
    pub created_at: u64,
    pub opportunities: Vec<MevOpportunity>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::U256;

    #[test]
    fn test_mev_opportunity_path_roundtrip() {
        use alloy::primitives::address;
        let opp = MevOpportunity {
            block_number: 1,
            tx_index: 0,
            strategy: Strategy::MultiHopArb,
            pool_a: address!("1111111111111111111111111111111111111111"),
            pool_b: address!("3333333333333333333333333333333333333333"),
            token_in: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            token_out: address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            input_amount: U256::from(1000u64),
            expected_profit: U256::from(100u64),
            gas_cost_wei: 1_000_000,
            timestamp: 12345,
            path: Some(vec![
                address!("1111111111111111111111111111111111111111"),
                address!("2222222222222222222222222222222222222222"),
                address!("3333333333333333333333333333333333333333"),
            ]),
            tick_lower: None,
            tick_upper: None,
            liquidity_amount: None,
            victim_tx_index: None,
            backrun_tx_index: None,
        };
        let json = serde_json::to_string(&opp).unwrap();
        let deserialized: MevOpportunity = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, opp.path);
        assert!(json.contains("\"path\""));
    }

    #[test]
    fn test_mev_opportunity_jit_fields_roundtrip() {
        use alloy::primitives::address;
        let opp = MevOpportunity {
            block_number: 1,
            tx_index: 5,
            strategy: Strategy::Jit,
            pool_a: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            pool_b: Address::ZERO,
            token_in: address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            token_out: address!("cccccccccccccccccccccccccccccccccccccccc"),
            input_amount: U256::from(0),
            expected_profit: U256::from(1000),
            gas_cost_wei: 0,
            timestamp: 12345,
            path: None,
            tick_lower: Some(-88720),
            tick_upper: Some(88720),
            liquidity_amount: Some(500_000u128),
            victim_tx_index: None,
            backrun_tx_index: None,
        };
        let json = serde_json::to_string(&opp).unwrap();
        let deserialized: MevOpportunity = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.tick_lower, Some(-88720));
        assert_eq!(deserialized.tick_upper, Some(88720));
        assert_eq!(deserialized.liquidity_amount, Some(500_000));
        assert!(json.contains("\"tick_lower\""));
        assert!(json.contains("\"tick_upper\""));
        assert!(json.contains("\"liquidity_amount\""));

        // Verify JIT fields are absent from serde output when None
        let no_jit = MevOpportunity {
            tick_lower: None,
            tick_upper: None,
            liquidity_amount: None,
            ..opp
        };
        let json_no = serde_json::to_string(&no_jit).unwrap();
        assert!(!json_no.contains("tick_lower"));
        assert!(!json_no.contains("tick_upper"));
        assert!(!json_no.contains("liquidity_amount"));
    }

    #[test]
    fn test_mev_opportunity_sandwich_fields_roundtrip() {
        use alloy::primitives::address;
        let opp = MevOpportunity {
            block_number: 1,
            tx_index: 0,
            strategy: Strategy::Sandwich,
            pool_a: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            pool_b: Address::ZERO,
            token_in: address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            token_out: address!("cccccccccccccccccccccccccccccccccccccccc"),
            input_amount: U256::from(1000),
            expected_profit: U256::from(500),
            gas_cost_wei: 0,
            timestamp: 12345,
            path: None,
            tick_lower: None,
            tick_upper: None,
            liquidity_amount: None,
            victim_tx_index: Some(1),
            backrun_tx_index: Some(2),
        };
        let json = serde_json::to_string(&opp).unwrap();
        let deserialized: MevOpportunity = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.victim_tx_index, Some(1));
        assert_eq!(deserialized.backrun_tx_index, Some(2));
        assert!(json.contains("\"victim_tx_index\""));
        assert!(json.contains("\"backrun_tx_index\""));

        // Verify fields are absent from serde output when None
        let no_sandwich = MevOpportunity {
            victim_tx_index: None,
            backrun_tx_index: None,
            ..opp
        };
        let json_no = serde_json::to_string(&no_sandwich).unwrap();
        assert!(!json_no.contains("victim_tx_index"));
    }
}
