use alloy::primitives::{Address, U256};
use crate::types::Strategy;

/// A detected MEV opportunity from backtesting.
#[derive(Debug, Clone)]
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
    /// Expected profit in USD (estimated)
    pub expected_profit_usd: f64,
    /// Estimated gas cost in USD
    pub gas_cost_usd: f64,
    /// Net profit after gas
    pub net_profit_usd: f64,
    /// Timestamp of the block
    pub timestamp: u64,
}
