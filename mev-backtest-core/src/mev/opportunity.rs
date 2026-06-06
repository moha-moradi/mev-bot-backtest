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
