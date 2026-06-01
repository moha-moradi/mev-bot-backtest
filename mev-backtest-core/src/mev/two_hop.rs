use alloy::primitives::{Address, U256};

use crate::mev::opportunity::MevOpportunity;
use crate::mev::pricing;
use crate::pool::math::optimal_two_hop_arb;
use crate::pool::state::{PoolManager, PoolState};
use crate::types::Strategy;

const GAS_UNITS: u64 = 200_000;

/// Detects two-hop arbitrage opportunities between V2 pools.
pub struct TwoHopArbDetector {
    pub min_profit_usd: f64,
}

impl TwoHopArbDetector {
    pub fn new(min_profit_usd: f64) -> Self {
        TwoHopArbDetector { min_profit_usd }
    }

    /// Detect arbitrage opportunities across all pool pairs in the manager.
    /// `base_fee_per_gas` in wei, `priority_fee_gwei` in gwei.
    pub fn detect(
        &self,
        pool_manager: &PoolManager,
        block_number: u64,
        tx_index: usize,
        timestamp: u64,
        base_fee_per_gas: u128,
        priority_fee_gwei: f64,
    ) -> Vec<MevOpportunity> {
        let mut opportunities = Vec::new();
        let pairs = pool_manager.arbitrage_pairs();

        for (pool_a, pool_b, shared_token) in &pairs {
            if let Some(opp) = self.check_direction(
                pool_manager, *pool_a, *pool_b, *shared_token,
                block_number, tx_index, timestamp,
                base_fee_per_gas, priority_fee_gwei,
            ) {
                opportunities.push(opp);
            }
            if let Some(opp) = self.check_direction(
                pool_manager, *pool_b, *pool_a, *shared_token,
                block_number, tx_index, timestamp,
                base_fee_per_gas, priority_fee_gwei,
            ) {
                opportunities.push(opp);
            }
        }

        opportunities
    }

    fn check_direction(
        &self,
        pm: &PoolManager,
        buy_pool: Address,
        sell_pool: Address,
        shared_token: Address,
        block_number: u64,
        tx_index: usize,
        timestamp: u64,
        base_fee_per_gas: u128,
        priority_fee_gwei: f64,
    ) -> Option<MevOpportunity> {
        let pool_a = pm.get(&buy_pool)?;
        let pool_b = pm.get(&sell_pool)?;

        let (r_a_other, r_a_shared, fee_a, token_in, _) =
            extract_v2_reserves_for_direction(pool_a, shared_token)?;

        let (r_b_in, r_b_out, fee_b, _, token_out) =
            extract_v2_reserves_for_sell_direction(pool_b, shared_token)?;

        let min_reserve = 1000u128;
        if r_a_other < min_reserve || r_a_shared < min_reserve
            || r_b_in < min_reserve || r_b_out < min_reserve
        {
            return None;
        }

        let result = optimal_two_hop_arb(r_a_other, r_a_shared, fee_a, r_b_in, r_b_out, fee_b)?;

        if result.profit == 0 {
            return None;
        }

        let profit_u256 = U256::from(result.profit);

        let gas_cost_wei = (GAS_UNITS as u128)
            .checked_mul(base_fee_per_gas + (priority_fee_gwei * 1e9) as u128)
            .unwrap_or(u128::MAX);
        let gas_cost_matic = gas_cost_wei as f64 / 1e18;
        let gas_cost_usd = gas_cost_matic * pricing::matic_usd_price();

        let expected_profit_usd = pricing::raw_amount_to_usd(token_out, result.profit)
            .unwrap_or(0.0);
        let net_profit_usd = expected_profit_usd - gas_cost_usd;

        if net_profit_usd < self.min_profit_usd {
            return None;
        }

        Some(MevOpportunity {
            block_number,
            tx_index,
            strategy: Strategy::TwoHopArb,
            pool_a: buy_pool,
            pool_b: sell_pool,
            token_in,
            token_out,
            input_amount: U256::from(result.input_amount),
            expected_profit: profit_u256,
            expected_profit_usd,
            gas_cost_usd,
            net_profit_usd,
            timestamp,
        })
    }
}

/// Extract reserves for buying a specific token from a pool.
fn extract_v2_reserves_for_direction(
    pool: &PoolState,
    shared_token: Address,
) -> Option<(u128, u128, u32, Address, Address)> {
    match pool {
        PoolState::UniswapV2(s) => {
            let fee = s.info.fee;
            if s.info.token0 == shared_token {
                Some((s.reserve1, s.reserve0, fee, s.info.token1, s.info.token0))
            } else if s.info.token1 == shared_token {
                Some((s.reserve0, s.reserve1, fee, s.info.token0, s.info.token1))
            } else {
                None
            }
        }
    }
}

/// Extract reserves for selling a specific token to a pool.
fn extract_v2_reserves_for_sell_direction(
    pool: &PoolState,
    shared_token: Address,
) -> Option<(u128, u128, u32, Address, Address)> {
    match pool {
        PoolState::UniswapV2(s) => {
            let fee = s.info.fee;
            if s.info.token0 == shared_token {
                Some((s.reserve0, s.reserve1, fee, s.info.token0, s.info.token1))
            } else if s.info.token1 == shared_token {
                Some((s.reserve1, s.reserve0, fee, s.info.token1, s.info.token0))
            } else {
                None
            }
        }
    }
}
