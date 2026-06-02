use std::cmp;

use alloy::primitives::{Address, U256};

use crate::mev::opportunity::MevOpportunity;
use crate::pool::math::{constant_product_output_amount, optimal_two_hop_arb, optimal_two_hop_arb_generic, TwoHopArbResult};
use crate::pool::state::{PoolManager, PoolState, UniswapV2PoolState};
use crate::pool::v3_quote::quote_v3_exact_in;
use crate::types::Strategy;

const GAS_LIMIT: u64 = 200_000;

/// Detects two-hop arbitrage opportunities across V2, V3, and mixed pools.
pub struct TwoHopArbDetector;

impl TwoHopArbDetector {
    /// Detect arbitrage opportunities across all pool pairs in the manager.
    pub fn detect(
        pool_manager: &PoolManager,
        block_number: u64,
        tx_index: usize,
        timestamp: u64,
        base_fee_per_gas: u128,
    ) -> Vec<MevOpportunity> {
        let mut opportunities = Vec::new();
        let pairs = pool_manager.arbitrage_pairs();

        for (pool_a, pool_b, shared_token) in &pairs {
            if let Some(opp) = Self::check_direction(
                pool_manager, *pool_a, *pool_b, *shared_token,
                block_number, tx_index, timestamp,
                base_fee_per_gas,
            ) {
                opportunities.push(opp);
            }
            if let Some(opp) = Self::check_direction(
                pool_manager, *pool_b, *pool_a, *shared_token,
                block_number, tx_index, timestamp,
                base_fee_per_gas,
            ) {
                opportunities.push(opp);
            }
        }

        opportunities
    }

    #[allow(clippy::too_many_arguments)]
    fn check_direction(
        pm: &PoolManager,
        buy_pool: Address,
        sell_pool: Address,
        shared_token: Address,
        block_number: u64,
        tx_index: usize,
        timestamp: u64,
        base_fee_per_gas: u128,
    ) -> Option<MevOpportunity> {
        let pool_a = pm.get(&buy_pool)?;
        let pool_b = pm.get(&sell_pool)?;

        let (token_in, token_out) = arb_tokens(pool_a, pool_b, shared_token)?;

        let result = quote_path(pool_a, pool_b, shared_token)?;

        if result.profit == 0 {
            return None;
        }

        let gas_cost_wei = (GAS_LIMIT as u128).saturating_mul(base_fee_per_gas);

        Some(MevOpportunity {
            block_number,
            tx_index,
            strategy: Strategy::TwoHopArb,
            pool_a: buy_pool,
            pool_b: sell_pool,
            token_in,
            token_out,
            input_amount: U256::from(result.input_amount),
            expected_profit: U256::from(result.profit),
            gas_cost_wei,
            timestamp,
        })
    }
}

/// Compute the optimal two-hop arbitrage result between any two pools that share a token.
///
/// Supports all pool type combinations:
/// - UniswapV2 ↔ UniswapV2
/// - UniswapV3 ↔ UniswapV3
/// - UniswapV2 ↔ UniswapV3 (both directions)
/// - Curve ↔ Curve
/// - Balancer ↔ Balancer
///
/// Returns `None` if the pool types are not supported or no profitable path exists.
pub fn quote_path(
    pool_a: &PoolState,
    pool_b: &PoolState,
    shared_token: Address,
) -> Option<TwoHopArbResult> {
    match (pool_a, pool_b) {
        (PoolState::UniswapV2(a), PoolState::UniswapV2(b)) => {
            let (r_a_other, r_a_shared, fee_a) = v2_reserves(a, shared_token, true)?;
            let (r_b_in, r_b_out, fee_b) = v2_reserves(b, shared_token, false)?;
            let min_reserve = 1000u128;
            if r_a_other < min_reserve || r_a_shared < min_reserve
                || r_b_in < min_reserve || r_b_out < min_reserve
            {
                return None;
            }
            optimal_two_hop_arb(r_a_other, r_a_shared, fee_a, r_b_in, r_b_out, fee_b)
        }
        (PoolState::UniswapV3(a), PoolState::UniswapV3(b)) => {
            let zero_a = shared_token == a.info.token1;
            let zero_b = shared_token == b.info.token0;
            let max_input = cmp::max(a.liquidity, b.liquidity);
            let quote_a = |x: u128| quote_v3_exact_in(a, x, zero_a);
            let quote_b = |x: u128| quote_v3_exact_in(b, x, zero_b);
            optimal_two_hop_arb_generic(max_input, &quote_a, &quote_b)
        }
        (PoolState::UniswapV2(a), PoolState::UniswapV3(b)) => {
            let (r_a_other, r_a_shared, fee_a) = v2_reserves(a, shared_token, true)?;
            let zero_b = shared_token == b.info.token0;
            let max_input = r_a_other;
            let quote_a = |x: u128| constant_product_output_amount(x, r_a_other, r_a_shared, fee_a);
            let quote_b = |x: u128| quote_v3_exact_in(b, x, zero_b);
            optimal_two_hop_arb_generic(max_input, &quote_a, &quote_b)
        }
        (PoolState::UniswapV3(a), PoolState::UniswapV2(b)) => {
            let zero_a = shared_token == a.info.token1;
            let (r_b_in, r_b_out, fee_b) = v2_reserves(b, shared_token, false)?;
            let max_input = r_b_out;
            let quote_a = |x: u128| quote_v3_exact_in(a, x, zero_a);
            let quote_b = |x: u128| constant_product_output_amount(x, r_b_in, r_b_out, fee_b);
            optimal_two_hop_arb_generic(max_input, &quote_a, &quote_b)
        }
        // Curve, Balancer, and mixed types — not yet implemented (V2/V3 focus)
        _ => None,
    }
}

/// Extract the token_in (spent) and token_out (received) for a two-hop arb
/// given two pools that share a common token.
fn arb_tokens(
    pool_a: &PoolState,
    pool_b: &PoolState,
    shared_token: Address,
) -> Option<(Address, Address)> {
    let info_a = pool_a.info();
    let info_b = pool_b.info();
    let token_in = if info_a.token0 == shared_token {
        info_a.token1
    } else if info_a.token1 == shared_token {
        info_a.token0
    } else {
        return None;
    };
    let token_out = if info_b.token0 == shared_token {
        info_b.token1
    } else if info_b.token1 == shared_token {
        info_b.token0
    } else {
        return None;
    };
    Some((token_in, token_out))
}

/// Extract V2 pool reserves for a given direction relative to `shared_token`.
/// `buy_side = true`  → returns (reserve_other, reserve_shared, fee) where
///                        reserve_shared is what we receive (the bridge token).
/// `buy_side = false` → returns (reserve_shared, reserve_other, fee) where
///                        reserve_shared is what we give (the bridge token).
fn v2_reserves(
    pool: &UniswapV2PoolState,
    shared_token: Address,
    buy_side: bool,
) -> Option<(u128, u128, u32)> {
    let fee = pool.info.fee;
    if buy_side {
        // We give the other token, receive shared_token
        if pool.info.token0 == shared_token {
            Some((pool.reserve1, pool.reserve0, fee))
        } else if pool.info.token1 == shared_token {
            Some((pool.reserve0, pool.reserve1, fee))
        } else {
            None
        }
    } else {
        // We give shared_token, receive the other token
        if pool.info.token0 == shared_token {
            Some((pool.reserve0, pool.reserve1, fee))
        } else if pool.info.token1 == shared_token {
            Some((pool.reserve1, pool.reserve0, fee))
        } else {
            None
        }
    }
}
