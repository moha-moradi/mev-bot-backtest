use alloy::primitives::{Address, U256};

use crate::mev::opportunity::MevOpportunity;
use crate::pool::math::optimal_two_hop_arb;
use crate::pool::state::{PoolManager, PoolState};
use crate::types::Strategy;

/// Detects two-hop arbitrage opportunities between V2 pools.
pub struct TwoHopArbDetector {
    pub min_profit_usd: f64,
}

impl TwoHopArbDetector {
    pub fn new(min_profit_usd: f64) -> Self {
        TwoHopArbDetector { min_profit_usd }
    }

    /// Detect arbitrage opportunities across all pool pairs in the manager.
    /// `block_number` and `tx_index` are used to tag the opportunities.
    pub fn detect(
        &self,
        pool_manager: &PoolManager,
        block_number: u64,
        tx_index: usize,
        timestamp: u64,
    ) -> Vec<MevOpportunity> {
        let mut opportunities = Vec::new();
        let pairs = pool_manager.arbitrage_pairs();

        for (pool_a, pool_b, shared_token) in &pairs {
            // Try both directions
            if let Some(opp) = self.check_direction(
                pool_manager, *pool_a, *pool_b, *shared_token,
                block_number, tx_index, timestamp,
            ) {
                opportunities.push(opp);
            }
            if let Some(opp) = self.check_direction(
                pool_manager, *pool_b, *pool_a, *shared_token,
                block_number, tx_index, timestamp,
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
    ) -> Option<MevOpportunity> {
        let pool_a = pm.get(&buy_pool)?;
        let pool_b = pm.get(&sell_pool)?;

        // Pool A: we BUY shared_token (pay other token, receive shared token)
        let (r_a_other, r_a_shared, fee_a, token_in, _) =
            extract_v2_reserves_for_direction(pool_a, shared_token)?;

        // Pool B: we SELL shared_token (give shared token, receive other token)
        let (r_b_in, r_b_out, fee_b, _, token_out) =
            extract_v2_reserves_for_sell_direction(pool_b, shared_token)?;

        // Check both pools have sufficient reserves
        let min_reserve = 1000u128;
        if r_a_other < min_reserve || r_a_shared < min_reserve
            || r_b_in < min_reserve || r_b_out < min_reserve
        {
            return None;
        }

        let result = optimal_two_hop_arb(r_a_other, r_a_shared, fee_a, r_b_in, r_b_out, fee_b)?;

        // Filter out zero-profit opportunities
        if result.profit == 0 {
            return None;
        }

        let profit_u256 = U256::from(result.profit);

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
            expected_profit_usd: 0.0,
            gas_cost_usd: 0.0,
            net_profit_usd: 0.0,
            timestamp,
        })
    }
}

/// Extract reserves for a specific direction through a pool.
/// Given a pool state and the token we want to buy, returns the reserves
/// of the pool from the perspective of buying that token.
///
/// Returns (reserve_in, reserve_out, fee, token_in, token_out)
/// where token_in is what we spend and token_out (shared_token) is what we get.
/// Extract reserves for buying a specific token from a pool.
///
/// Returns (other_reserve, shared_reserve, fee, other_token, shared_token)
/// where we spend `other_token` to buy `shared_token`.
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
///
/// Returns (shared_reserve, other_reserve, fee, shared_token, other_token)
/// where we give `shared_token` to get `other_token`.
fn extract_v2_reserves_for_sell_direction(
    pool: &PoolState,
    shared_token: Address,
) -> Option<(u128, u128, u32, Address, Address)> {
    match pool {
        PoolState::UniswapV2(s) => {
            let fee = s.info.fee;
            if s.info.token0 == shared_token {
                // We give token0 (shared), get token1 (other)
                Some((s.reserve0, s.reserve1, fee, s.info.token0, s.info.token1))
            } else if s.info.token1 == shared_token {
                // We give token1 (shared), get token0 (other)
                Some((s.reserve1, s.reserve0, fee, s.info.token1, s.info.token0))
            } else {
                None
            }
        }
    }
}
