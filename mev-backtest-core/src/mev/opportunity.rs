use alloy::primitives::{Address, U256};
use serde::{Deserialize, Serialize};
use crate::types::Strategy;

/// A detected MEV opportunity from backtesting.
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
}

/// Saved results file wrapping opportunities with run metadata.
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
        };
        let json = serde_json::to_string(&opp).unwrap();
        let deserialized: MevOpportunity = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, opp.path);
        assert!(json.contains("\"path\""));
    }
}
