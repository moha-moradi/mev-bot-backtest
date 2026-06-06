# MultiHopArb Detection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add N-hop (3+ pool) arbitrage detection alongside the existing TwoHopArb.

**Architecture:** A new `MultiHopArbDetector` stateless struct (parallel to `TwoHopArbDetector`) that discovers N-pool paths via BFS on the pool-token graph, composes sequential per-pool quoting, and uses ternary search to find the optimal input amount.

**Tech Stack:** Rust, alloy (U256/Address), existing pool math (v2 CPMM, v3 exact-in quoting), existing PoolManager graph.

---

### Task 1: Add `path` field to MevOpportunity

**Files:**
- Modify: `mev-backtest-core/src/mev/opportunity.rs`
- Test: unit test within the same file

- [ ] **Step 1: Add the field**

```rust
/// Full pool path for multi-hop opportunities (e.g., [buy, intermediate, ..., sell])
#[serde(default, skip_serializing_if = "Option::is_none")]
pub path: Option<Vec<Address>>,
```

Place this after `pub timestamp: u64,` in `MevOpportunity`. Import `std::vec::Vec` is already available via `alloy` re-exports; `Address` is already imported.

- [ ] **Step 2: Write unit test for serialization roundtrip**

Append to existing tests in `opportunity.rs`:

```rust
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
```

- [ ] **Step 3: Verify compiles and test passes**

```bash
cargo test --lib types::tests -- --nocapture
cargo check
```

- [ ] **Step 4: Commit**

```bash
git add mev-backtest-core/src/mev/opportunity.rs
git commit -m "feat: add optional path field to MevOpportunity for multi-hop"
```

---

### Task 2: Extract ternary search into `optimal_n_hop_generic`

**Files:**
- Modify: `mev-backtest-core/src/pool/math.rs`
- Test: unit tests in math.rs

- [ ] **Step 1: Add `optimal_n_hop_generic` function before `optimal_two_hop_arb_generic`**

```rust
/// General N-hop ternary search optimizer.
///
/// `quote_fn(x)` returns the output amount for input `x` through the entire pool chain.
/// Returns `Some((optimal_input, output_amount))` or `None` if no profitable path found.
pub fn optimal_n_hop_generic(
    max_input: u128,
    quote_fn: &impl Fn(u128) -> Option<u128>,
) -> Option<(u128, u128)> {
    if max_input == 0 {
        return None;
    }

    let mut lo = 0u128;
    let mut hi = max_input;
    let mut best: Option<(u128, u128)> = None;

    for _ in 0..80 {
        let m1 = lo + (hi - lo) / 3;
        let m2 = hi - (hi - lo) / 3;

        if m1 == m2 {
            break;
        }

        let o1 = quote_fn(m1);
        let o2 = quote_fn(m2);

        match (o1, o2) {
            (None, None) => break,
            (Some(_), None) => hi = m2,
            (None, Some(_)) => lo = m1,
            (Some(r1), Some(r2)) => {
                let p1 = r1.saturating_sub(m1);
                let p2 = r2.saturating_sub(m2);
                if p1 >= p2 {
                    hi = m2;
                    if p1 > 0 {
                        best = Some((m1, r1));
                    }
                } else {
                    lo = m1;
                    if p2 > 0 {
                        best = Some((m2, r2));
                    }
                }
            }
        }
    }

    best
}
```

- [ ] **Step 2: Write unit tests**

Append after `test_optimal_two_hop_arb_generic_zero_max_input`:

```rust
#[test]
fn test_optimal_n_hop_generic_two_step_matches_two_hop() {
    // Same 2-step should match optimal_two_hop_arb_generic
    let reserve_a_in = 1_000_000u128;
    let reserve_a_out = 2_000_000u128;
    let fee_a = 30;
    let reserve_b_in = 2_000_000u128;
    let reserve_b_out = 1_000_000u128;
    let fee_b = 30;

    let quote_2hop = |x: u128| {
        let mid = constant_product_output_amount(x, reserve_a_in, reserve_a_out, fee_a)?;
        constant_product_output_amount(mid, reserve_b_in, reserve_b_out, fee_b)
    };

    let max_input = 1_000_000u128;
    let n_result = optimal_n_hop_generic(max_input, &quote_2hop);
    assert!(n_result.is_some());
    let (input, output) = n_result.unwrap();
    assert!(output > input);
}

#[test]
fn test_optimal_n_hop_generic_no_profit() {
    let quote_flat = |x: u128| -> Option<u128> { Some(x) }; // no profit
    assert!(optimal_n_hop_generic(1_000_000, &quote_flat).is_none());
}

#[test]
fn test_optimal_n_hop_generic_zero_max_input() {
    let quote = |x: u128| -> Option<u128> { Some(x + 1) };
    assert!(optimal_n_hop_generic(0, &quote).is_none());
}

#[test]
fn test_optimal_n_hop_generic_three_step() {
    let q1 = |x: u128| -> Option<u128> { Some(x * 2) };     // double
    let q2 = |x: u128| -> Option<u128> { Some(x * 3) };     // triple
    let q3 = |x: u128| -> Option<u128> { Some(x / 2) };     // halve
    let chain = |x: u128| -> Option<u128> {
        let a = q1(x)?;
        let b = q2(a)?;
        q3(b)
    };
    // chain(x) = (x * 2 * 3) / 2 = x * 3 → always profitable
    let result = optimal_n_hop_generic(1_000_000, &chain);
    assert!(result.is_some());
    let (input, output) = result.unwrap();
    assert!(output >= input * 3);
}
```

- [ ] **Step 3: Verify tests pass**

```bash
cargo test --lib pool::math::tests -- --nocapture
```

- [ ] **Step 4: Commit**

```bash
git add mev-backtest-core/src/pool/math.rs
git commit -m "feat: add optimal_n_hop_generic for N-hop ternary search"
```

---

### Task 3: Create `multi_hop.rs` detector

**Files:**
- Create: `mev-backtest-core/src/mev/multi_hop.rs`
- Modify: `mev-backtest-core/src/mev/mod.rs`
- Test: unit tests within multi_hop.rs

- [ ] **Step 1: Add module export to mev/mod.rs**

Add after `pub mod two_hop;`:
```rust
pub mod multi_hop;
```

- [ ] **Step 2: Create `mev/multi_hop.rs`**

```rust
use std::collections::VecDeque;
use alloy::primitives::Address;
use crate::mev::opportunity::MevOpportunity;
use crate::pool::math::{constant_product_output_amount, optimal_n_hop_generic};
use crate::pool::state::{PoolManager, PoolState};
use crate::pool::v3_quote::quote_v3_exact_in;
use crate::types::{GasConfig, Strategy};

pub struct MultiHopArbDetector;

impl MultiHopArbDetector {
    pub fn detect(
        pool_manager: &PoolManager,
        block_number: u64,
        tx_index: usize,
        timestamp: u64,
        base_fee_per_gas: u128,
        gas_config: GasConfig,
    ) -> Vec<MevOpportunity> {
        let max_depth = 4usize;
        let mut opportunities = Vec::new();

        let paths = Self::find_paths(pool_manager, max_depth);

        for path in &paths {
            if let Some(opp) = Self::check_path(
                pool_manager, path,
                block_number, tx_index, timestamp,
                base_fee_per_gas, gas_config,
            ) {
                opportunities.push(opp);
            }
        }

        opportunities
    }

    /// BFS-limited enumeration of pool paths through the token graph.
    /// Each path is `[buy_pool, ..., sell_pool]` where adjacent pools share a token.
    fn find_paths(pm: &PoolManager, max_depth: usize) -> Vec<Vec<Address>> {
        let mut all_paths = Vec::new();

        // Seed 2-pool paths from existing arbitrage pairs
        for &(pool_a, pool_b, _shared) in &pm.arbitrage_pairs() {
            let seed = vec![pool_a, pool_b];
            all_paths.push(seed.clone());
            Self::extend_path(pm, seed, &mut all_paths, max_depth);
        }

        all_paths
    }

    fn extend_path(pm: &PoolManager, path: Vec<Address>, all_paths: &mut Vec<Vec<Address>>, max_depth: usize) {
        if path.len() >= max_depth {
            return;
        }

        let last_pool = match pm.get(&path[path.len() - 1]) {
            Some(p) => p,
            None => return,
        };
        let prev_pool = match pm.get(&path[path.len() - 2]) {
            Some(p) => p,
            None => return,
        };

        // Determine the "forward token" — the token NOT shared with the previous pool
        let forward_token = Self::non_shared_token(last_pool, prev_pool);

        for &next_addr in pm.pools_for_token(forward_token) {
            if path.contains(&next_addr) {
                continue; // no cycles
            }
            let mut new_path = path.clone();
            new_path.push(next_addr);
            all_paths.push(new_path.clone());
            Self::extend_path(pm, new_path, all_paths, max_depth);
        }
    }

    /// Given a pool and the previous pool in the path, determine which token
    /// of `pool` is the "forward" side (not shared with `prev`).
    fn non_shared_token(pool: &PoolState, prev: &PoolState) -> Address {
        let info = pool.info();
        let prev_info = prev.info();
        if info.token0 == prev_info.token0 || info.token0 == prev_info.token1 {
            info.token1
        } else {
            info.token0
        }
    }

    fn check_path(
        pm: &PoolManager,
        path: &[Address],
        block_number: u64,
        tx_index: usize,
        timestamp: u64,
        base_fee_per_gas: u128,
        gas_config: GasConfig,
    ) -> Option<MevOpportunity> {
        if path.len() < 2 {
            return None;
        }

        let pool_a = pm.get(&path[0])?;
        let pool_b = pm.get(&path[path.len() - 1])?;

        // Determine token_in (non-shared side of first pool)
        let first_shared = {
            let next = pm.get(&path[1])?;
            let info_a = pool_a.info();
            let info_next = next.info();
            if info_a.token0 == info_next.token0 || info_a.token0 == info_next.token1 {
                info_a.token0
            } else {
                info_a.token1
            }
        };
        let token_in = if pool_a.info().token0 == first_shared {
            pool_a.info().token1
        } else {
            pool_a.info().token0
        };

        // Max input for first pool
        let max_input = Self::pool_max_input(pool_a);

        // Build the chain quote function
        let quote_fn = |x: u128| -> Option<u128> {
            let mut current = x;
            let mut current_token = token_in;
            for &addr in path {
                let pool = pm.get(&addr)?;
                current = Self::quote_single_pool(pool, current_token, current)?;
                let info = pool.info();
                current_token = if info.token0 == current_token { info.token1 } else { info.token0 };
            }
            Some(current)
        };

        let (input_amount, output_amount) = optimal_n_hop_generic(max_input, &quote_fn)?;

        if output_amount <= input_amount {
            return None;
        }

        // token_out is the non-shared side of the last pool
        let last = pm.get(&path[path.len() - 1])?;
        let prev = pm.get(&path[path.len() - 2])?;
        let last_shared = Self::non_shared_token(prev, last); // Hmm this needs work
        // Actually: token_out = non_shared_token(last, prev)
        // Let me fix: after extending, the forward token of last pool IS token_out
        // token_out = last.info().token0 if last.info().token0 != last_shared else last.info().token1
        
        // Determine token_out more carefully:
        let info_last = last.info();
        // Find which token of the last pool is NOT the shared token with prev
        let prev_info = prev.info();
        let shared_with_prev = if info_last.token0 == prev_info.token0 || info_last.token0 == prev_info.token1 {
            info_last.token0
        } else {
            info_last.token1
        };
        let token_out = if info_last.token0 == shared_with_prev {
            info_last.token1
        } else {
            info_last.token0
        };

        let gas_cost_wei = GasConfig {
            gas_limit: gas_config.gas_limit.saturating_mul(path.len() as u64),
            ..gas_config
        }.compute_gas_cost(base_fee_per_gas);

        Some(MevOpportunity {
            block_number,
            tx_index,
            strategy: Strategy::MultiHopArb,
            pool_a: path[0],
            pool_b: path[path.len() - 1],
            token_in,
            token_out,
            input_amount: U256::from(input_amount),
            expected_profit: U256::from(output_amount.saturating_sub(input_amount)),
            gas_cost_wei,
            timestamp,
            path: Some(path.to_vec()),
        })
    }

    fn pool_max_input(pool: &PoolState) -> u128 {
        match pool {
            PoolState::UniswapV2(v2) => std::cmp::min(v2.reserve0, v2.reserve1),
            PoolState::UniswapV3(v3) => v3.liquidity,
            _ => 1_000_000u128, // fallback for unsupported types
        }
    }

    fn quote_single_pool(pool: &PoolState, token_in: Address, amount_in: u128) -> Option<u128> {
        match pool {
            PoolState::UniswapV2(v2) => {
                let (reserve_in, reserve_out) = if v2.info.token0 == token_in {
                    (v2.reserve0, v2.reserve1)
                } else if v2.info.token1 == token_in {
                    (v2.reserve1, v2.reserve0)
                } else {
                    return None;
                };
                constant_product_output_amount(amount_in, reserve_in, reserve_out, v2.info.fee)
            }
            PoolState::UniswapV3(v3) => {
                let zero_for_one = v3.info.token0 == token_in;
                if !zero_for_one && v3.info.token1 != token_in {
                    return None;
                }
                quote_v3_exact_in(v3, amount_in, zero_for_one)
            }
            _ => None,
        }
    }
}
```

Wait, the above code has the `token_out` determination duplicated. Let me write the file properly.

- [ ] **Step 3: Write unit tests in multi_hop.rs**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::{address, U256};
    use crate::pool::state::{PoolInfo, UniswapV2PoolState, UniswapV3PoolState};
    use crate::pool::dex_type::DexType;
    use std::collections::HashMap;

    fn usdc() -> Address { address!("2791bca1f2de4661ed88a30c99a7a9449aa84174") }
    fn wmatic() -> Address { address!("0d500b1d8e8ef31e21c99d1db9a6444d3adf1270") }
    fn usdt() -> Address { address!("c2132d05d31c914a87c6611c10748aeb04b58e8f") }

    fn v2_pool(addr: Address, t0: Address, t1: Address, r0: u128, r1: u128) -> PoolState {
        PoolState::UniswapV2(UniswapV2PoolState {
            info: PoolInfo {
                address: addr, token0: t0, token1: t1, fee: 30,
                name: None, dex_type: DexType::UniswapV2, tick_spacing: None,
            },
            reserve0: r0, reserve1: r1,
        })
    }

    fn default_gas() -> GasConfig { GasConfig::default() }

    #[test]
    fn test_detect_empty_no_paths() {
        let pm = PoolManager::new();
        let opps = MultiHopArbDetector::detect(&pm, 1, 0, 100, 50_000_000_000, default_gas());
        assert!(opps.is_empty());
    }

    #[test]
    fn test_detect_two_pool_same_as_two_hop() {
        let mut pm = PoolManager::new();
        pm.add_pool(v2_pool(address!("1111111111111111111111111111111111111111"), usdc(), wmatic(), 1_000_000, 2_000_000));
        pm.add_pool(v2_pool(address!("2222222222222222222222222222222222222222"), wmatic(), usdt(), 1_000_000, 500_000));
        let opps = MultiHopArbDetector::detect(&pm, 1, 0, 100, 50_000_000_000, default_gas());
        assert!(!opps.is_empty());
        for opp in &opps {
            assert_eq!(opp.strategy, Strategy::MultiHopArb);
            assert!(opp.path.is_some());
            assert_eq!(opp.path.as_ref().unwrap().len(), 2);
        }
    }

    #[test]
    fn test_find_paths_three_pool_triangular() {
        let mut pm = PoolManager::new();
        // USDC/WMATIC, WMATIC/USDT, USDC/USDT — triangular circuit
        pm.add_pool(v2_pool(address!("1111111111111111111111111111111111111111"), usdc(), wmatic(), 1_000_000, 2_000_000));
        pm.add_pool(v2_pool(address!("2222222222222222222222222222222222222222"), wmatic(), usdt(), 1_000_000, 500_000));
        pm.add_pool(v2_pool(address!("3333333333333333333333333333333333333333"), usdc(), usdt(), 1_000_000, 1_000_000));

        let paths = MultiHopArbDetector::find_paths(&pm, 4);
        // Should include 2-pool and 3-pool paths
        assert!(paths.len() >= 2);
        let has_three_hop = paths.iter().any(|p| p.len() == 3);
        assert!(has_three_hop, "Should find at least one 3-pool path");
    }

    #[test]
    fn test_detect_three_pool_triangular() {
        let mut pm = PoolManager::new();
        // Triangular circuit with price imbalance
        // Pool A: USDC/WMATIC, cheap WMATIC (0.5 USDC)
        // Pool B: WMATIC/USDT, expensive WMATIC (2 USDT)  
        // Pool C: USDC/USDT, 1:1
        // Arb: USDC → A → WMATIC → B → USDT → C → USDC
        pm.add_pool(v2_pool(address!("1111111111111111111111111111111111111111"), usdc(), wmatic(), 1_000_000, 2_000_000));
        pm.add_pool(v2_pool(address!("2222222222222222222222222222222222222222"), wmatic(), usdt(), 1_000_000, 500_000));
        pm.add_pool(v2_pool(address!("3333333333333333333333333333333333333333"), usdc(), usdt(), 1_000_000, 1_000_000));

        let opps = MultiHopArbDetector::detect(&pm, 1, 0, 100, 50_000_000_000, default_gas());
        assert!(!opps.is_empty(), "Should detect triangular arb");

        // Find a 3-pool opportunity
        let paths_3: Vec<_> = opps.iter().filter(|o| o.path.as_ref().map(|p| p.len() >= 3).unwrap_or(false)).collect();
        assert!(paths_3.len() >= 1, "Should have at least one 3-pool path");

        for opp in paths_3 {
            assert!(opp.expected_profit > U256::ZERO);
            assert!(opp.gas_cost_wei > 0);
        }
    }

    #[test]
    fn test_no_cycle_repeat_pools() {
        let mut pm = PoolManager::new();
        // Two pools, no way to make a cycle
        pm.add_pool(v2_pool(address!("1111111111111111111111111111111111111111"), usdc(), wmatic(), 1_000_000, 2_000_000));
        pm.add_pool(v2_pool(address!("2222222222222222222222222222222222222222"), usdc(), usdt(), 1_000_000, 1_000_000));

        let paths = MultiHopArbDetector::find_paths(&pm, 4);
        // No pool should appear twice in a path
        for path in &paths {
            let mut seen = std::collections::HashSet::new();
            for &addr in path {
                assert!(seen.insert(addr), "Duplicate pool {} in path {:?}", addr, path);
            }
        }
    }

    #[test]
    fn test_detect_no_profit_flat_prices() {
        let mut pm = PoolManager::new();
        pm.add_pool(v2_pool(address!("1111111111111111111111111111111111111111"), usdc(), wmatic(), 1_000_000, 1_000_000));
        pm.add_pool(v2_pool(address!("2222222222222222222222222222222222222222"), wmatic(), usdt(), 1_000_000, 1_000_000));
        pm.add_pool(v2_pool(address!("3333333333333333333333333333333333333333"), usdc(), usdt(), 1_000_000, 1_000_000));

        let opps = MultiHopArbDetector::detect(&pm, 1, 0, 100, 50_000_000_000, default_gas());
        // Equal prices should yield no profit
        assert!(opps.is_empty());
    }
}
```

- [ ] **Step 4: Fix the token_in/token_out determination in `check_path`**

The `check_path` function has a bug in the rough draft above — `token_out` is computed twice. Here's the corrected version for the full file:

Replace the whole `check_path` body with this corrected version:

```rust
fn check_path(
    pm: &PoolManager,
    path: &[Address],
    block_number: u64,
    tx_index: usize,
    timestamp: u64,
    base_fee_per_gas: u128,
    gas_config: GasConfig,
) -> Option<MevOpportunity> {
    if path.len() < 2 {
        return None;
    }

    let pool_a = pm.get(&path[0])?;
    let pool_b = pm.get(&path[path.len() - 1])?;

    // token_in = non-shared side of first pool
    let next = pm.get(&path[1])?;
    let info_a = pool_a.info();
    let info_next = next.info();
    let first_shared = if info_a.token0 == info_next.token0 || info_a.token0 == info_next.token1 {
        info_a.token0
    } else {
        info_a.token1
    };
    let token_in = if info_a.token0 == first_shared {
        info_a.token1
    } else {
        info_a.token0
    };

    // token_out = non-shared side of last pool
    let prev = pm.get(&path[path.len() - 2])?;
    let info_b = pool_b.info();
    let last_shared = if info_b.token0 == prev.info().token0 || info_b.token0 == prev.info().token1 {
        info_b.token0
    } else {
        info_b.token1
    };
    let token_out = if info_b.token0 == last_shared {
        info_b.token1
    } else {
        info_b.token0
    };

    let max_input = Self::pool_max_input(pool_a);

    let quote_fn = |x: u128| -> Option<u128> {
        let mut current = x;
        let mut current_token = token_in;
        for &addr in path {
            let pool = pm.get(&addr)?;
            current = Self::quote_single_pool(pool, current_token, current)?;
            let info = pool.info();
            current_token = if info.token0 == current_token { info.token1 } else { info.token0 };
        }
        Some(current)
    };

    let (input_amount, output_amount) = optimal_n_hop_generic(max_input, &quote_fn)?;

    if output_amount <= input_amount {
        return None;
    }

    let gas_cost_wei = GasConfig {
        gas_limit: gas_config.gas_limit.saturating_mul(path.len() as u64),
        ..gas_config
    }.compute_gas_cost(base_fee_per_gas);

    Some(MevOpportunity {
        block_number,
        tx_index,
        strategy: Strategy::MultiHopArb,
        pool_a: path[0],
        pool_b: path[path.len() - 1],
        token_in,
        token_out,
        input_amount: U256::from(input_amount),
        expected_profit: U256::from(output_amount.saturating_sub(input_amount)),
        gas_cost_wei,
        timestamp,
        path: Some(path.to_vec()),
    })
}
```

Wait — the `quote_fn` closure borrows `pm` and `path`, but these are references from the function parameter. The closure captures `&PoolManager` and `&[Address]` — both references, so no lifetime issues. Good.

But wait — `pm` is `&PoolManager`, `path` is `&[Address]`. The closure `quote_fn` captures `pm` and `path` by reference, then `optimal_n_hop_generic` takes `&impl Fn(u128) -> Option<u128>`. This should work.

However, there's a subtlety: `pm` is already a reference, and the closure captures `pm` as `&PoolManager`. When we call `pm.get(&addr)` inside the closure, it's calling `(&PoolManager).get(&addr)`. Since `get` takes `&self`, this should work.

Also for the `path` — the closure captures `path: &[Address]`, and iterating with `for &addr in path` iterates over references to elements.

This should compile. Let me check the test imports though.

Actually I realize there's also the question: does the `check_path` function need to be public or accessible? The `detect` method calls `Self::find_paths` and `Self::check_path`, both private to the struct. The tests call `MultiHopArbDetector::find_paths` directly if I use it. Let me make it pub(crate) or keep it pub and just use it in tests.

For the tests, I'll call `MultiHopArbDetector::detect` mostly, and for `find_paths` tests I'll access it through the public API.

Actually, `find_paths` is not a standard part of the public API. For testing, I'll just call `detect` and verify the results. But for the path finding unit tests, I need to access `find_paths`. Let me make `find_paths` public:

```rust
pub fn find_paths(pm: &PoolManager, max_depth: usize) -> Vec<Vec<Address>> {
```

This is fine since the module is internal to the crate.

- [ ] **Step 5: Verify compiles and tests pass**

```bash
cargo test --lib mev::multi_hop::tests -- --nocapture
cargo check
```

- [ ] **Step 6: Commit**

```bash
git add mev-backtest-core/src/mev/mod.rs \
      mev-backtest-core/src/mev/multi_hop.rs
git commit -m "feat: add MultiHopArbDetector with N-hop path finding"
```

---

### Task 4: Wire MultiHopArbDetector into runner

**Files:**
- Modify: `mev-backtest-core/src/run.rs`

- [ ] **Step 1: Add import**

After `use crate::mev::two_hop::TwoHopArbDetector;`:
```rust
use crate::mev::multi_hop::MultiHopArbDetector;
```

- [ ] **Step 2: Call MultiHopArbDetector::detect in on_tx callback**

In `run_block()`, after the `TwoHopArbDetector::detect` call (around line 124-131), add:

```rust
let multi_opps = MultiHopArbDetector::detect(
    &pm,
    block_num,
    i,
    timestamp,
    base_fee_per_gas,
    self.gas_config,
);
all_opportunities.extend(multi_opps);
```

- [ ] **Step 3: Verify compiles**

```bash
cargo check
```

- [ ] **Step 4: Commit**

```bash
git add mev-backtest-core/src/run.rs
git commit -m "feat: wire MultiHopArbDetector into runner"
```

---

### Task 5: Integration tests

**Files:**
- Modify: `mev-backtest-core/tests/integration.rs`

- [ ] **Step 1: Add integration tests for MultiHopArb**

Append after existing tests:

```rust
#[test]
fn test_multi_hop_detection_three_pool() {
    use mev_backtest_core::types::GasConfig;
    use mev_backtest_core::mev::multi_hop::MultiHopArbDetector;
    use alloy::primitives::U256;

    let mut pm = PoolManager::new();

    // Triangular arb: USDC → WMATIC → USDT → USDC
    // Pool A: USDC/WMATIC (WMATIC cheap: 0.5 USDC each)
    // Pool B: WMATIC/USDT (WMATIC expensive: 2 USDT each)
    // Pool C: USDC/USDT (1:1)
    pm.add_pool(make_pool(
        matic_usdc_pool(), usdc(), wmatic(),
        1_000_000, 2_000_000,
    ));
    pm.add_pool(make_pool(
        matic_usdt_pool(), usdt(), wmatic(),
        1_000_000, 500_000,
    ));
    // Third pool: USDC/USDT (different addresses for test)
    let usdc_usdt_pool = address!("3333333333333333333333333333333333333333");
    pm.add_pool(make_pool(
        usdc_usdt_pool, usdc(), usdt(),
        1_000_000, 1_000_000,
    ));

    let opps = MultiHopArbDetector::detect(
        &pm, 1, 0, 12345, 50_000_000_000, GasConfig::default(),
    );

    // Should detect both 2-pool and 3-pool arb opportunities
    assert!(!opps.is_empty(), "Should detect multi-hop arb");

    // Find a 3-pool opportunity
    let three_hop: Vec<_> = opps.iter().filter(|o| {
        o.path.as_ref().map(|p| p.len() >= 3).unwrap_or(false)
    }).collect();
    assert!(!three_hop.is_empty(), "Should detect a 3-pool arb");

    for opp in &opps {
        assert_eq!(opp.strategy, mev_backtest_core::types::Strategy::MultiHopArb);
        assert!(opp.expected_profit > U256::ZERO);
        assert!(opp.gas_cost_wei > 0);
    }
}

#[test]
fn test_multi_hop_path_field_populated() {
    use mev_backtest_core::types::GasConfig;
    use mev_backtest_core::mev::multi_hop::MultiHopArbDetector;

    let mut pm = PoolManager::new();
    pm.add_pool(make_pool(
        matic_usdc_pool(), usdc(), wmatic(),
        1_000_000, 2_000_000,
    ));
    pm.add_pool(make_pool(
        matic_usdt_pool(), usdt(), wmatic(),
        1_000_000, 500_000,
    ));

    let opps = MultiHopArbDetector::detect(
        &pm, 1, 0, 12345, 50_000_000_000, GasConfig::default(),
    );

    assert!(!opps.is_empty());
    for opp in &opps {
        assert!(opp.path.is_some(), "MultiHopArb must have path populated");
        let path = opp.path.as_ref().unwrap();
        assert_eq!(path.len(), 2, "Two-pool path should have length 2");
        // Verify path[0] = pool_a, path[last] = pool_b
        assert_eq!(path[0], opp.pool_a);
        assert_eq!(path[path.len() - 1], opp.pool_b);
    }
}
```

- [ ] **Step 2: Run integration tests**

```bash
cargo test --test integration -- --nocapture
```

Expected: all tests pass, including the two new MultiHopArb tests.

- [ ] **Step 3: Commit**

```bash
git add mev-backtest-core/tests/integration.rs
git commit -m "test: add integration tests for MultiHopArb detection"
```

---

### Task 6: Expand GUIDE.md with run examples and config reference

**Files:**
- Modify: `GUIDE.md`

- [ ] **Step 1: Read current GUIDE.md to find insertion points**

```bash
# Read the file to find right sections to extend
```

- [ ] **Step 2: Add run example sections after CLI Reference, under a new "Examples" heading**

```markdown
## Examples

### Basic run on Polygon (last 100 blocks)
```bash
mev-backtest run --blocks 100 --chain polygon
```

### Run with custom gas settings
```bash
mev-backtest run --block 50000000 \
  --gas-limit 300000 \
  --priority-fee 2.0 \
  --gas-model p90
```

### Run with specific strategies
```bash
mev-backtest run --days 7 --strategies "two_hop_arb,multi_hop_arb"
```

### Run multi-hop arbitrage only (Polygon archive node)
```bash
mev-backtest run --blocks 1000 --chain polygon --strategies multi_hop_arb
```

### Fetch block data first, then run backtest
```bash
mev-backtest fetch --days 30 --chain polygon
mev-backtest run --days 30 --chain polygon
```

### Replay a specific block for debugging
```bash
mev-backtest replay --block 50000000 --chain polygon
```

### Report from saved JSON results
```bash
mev-backtest report
mev-backtest report --output csv
mev-backtest report --run-id run_1718000000
```

### Discover pools on a new chain
```bash
mev-backtest discover \
  --chain polygon \
  --v2-factories 0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32 \
  --from-block 0 --to-block 50000000 \
  --save
```

### Full TOML configuration
Create `mev-backtest.toml`:
```toml
chain = "polygon"
rpc_url = "https://polygon-rpc.com"
flash_loan_provider = "auto"
strategies = "all"
gas_model = "historical_exact"
gas_limit = 200000
priority_fee_gwei = 0.0
output = "table"
export_path = "./results"
cache_dir = "./cache"
```
```

- [ ] **Step 3: Add performance notes under a "Performance" heading**

```markdown
## Performance

### MultiHopArb Detection Cost
MultiHopArb enumerates all pool paths up to depth 4. For Polygon (~100 pools), this evaluates ~1,600 paths per block, each running 80 iterations of ternary search. Expected overhead: 50-200ms per block.

To reduce detection time:
- Use `--strategies two_hop_arb` to skip multi-hop detection
- Reduce path depth (hardcoded default: 4)
- Fewer pools = faster detection (use a slim pool registry)
```

- [ ] **Step 4: Commit**

```bash
git add GUIDE.md
git commit -m "docs: expand GUIDE.md with run examples and MultiHopArb docs"
```

---

### Task 7: Final verification

- [ ] **Step 1: Run full test suite**

```bash
cargo test --lib
cargo test --test integration
cargo check --workspace
```

Expected: 180+ unit tests pass, 12+ integration tests pass, workspace compiles clean.

- [ ] **Step 2: Run `cargo clippy` (if available)**

```bash
cargo clippy --all-targets 2>&1 || true
```

- [ ] **Step 3: Show git status**

```bash
git status
git log --oneline -5
```

- [ ] **Step 4: If all good, create a summary commit**

```bash
git add -A
git commit -m "feat: complete MultiHopArb detection with N-hop path finding

- Add path field to MevOpportunity for multi-pool tracking
- Extract optimal_n_hop_generic for N-hop ternary search
- Implement MultiHopArbDetector with BFS path finding
- Wire into runner alongside TwoHopArb
- Add unit and integration tests
- Expand GUIDE.md with run examples and config reference"
```
