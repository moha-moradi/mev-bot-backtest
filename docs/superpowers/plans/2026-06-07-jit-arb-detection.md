# JIT Arbitrage Detection — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Detect JIT liquidity + arbitrage combo (JitArb): same EOA mints concentrated liquidity on V3 pool P, then swaps through pool P and a different pool Q within the same block.

**Architecture:** A lightweight per-block `JitArbDetector` that tracks V3 Mint events and V3 Swap events (pool address + sender). On each `detect()`, matches active mints against same-sender swaps on the same pool and a different pool sharing a token. No new `MevOpportunity` fields needed — reuses `tick_lower/upper`, `liquidity_amount`, `pool_a/pool_b`.

**Tech Stack:** Rust, alloy, existing V3 event decoders, existing `PoolManager::get()` for token sharing check.

---

### Task 1: Create `mev/jit_arb.rs` — JitArbDetector + 6 unit tests

**Files:**
- Create: `mev-backtest-core/src/mev/jit_arb.rs`
- Modify: `mev-backtest-core/src/mev/mod.rs`

**Design summary:**
- `SwapEvent { tx_index, pool, sender }` — lightweight V3 swap record
- `JitArbMint { mint_tx_index, tick_lower, tick_upper, amount, sender, swapped, burned }` — active mint
- `JitArbDetector { active_mints, swap_events, emitted, block_number }`
- `process_tx(tx_index, logs, sender)` — decodes V3 Mint/Burn/Swap events, updates state
- `detect(timestamp, pool_manager)` — cross-pool matching: for each active mint, find same-sender swaps on JIT pool and on a different pool sharing a token

- [ ] **Step 1: Add module export to mev/mod.rs**

Add after `pub mod sandwich;`:
```rust
pub mod jit_arb;
```

- [ ] **Step 2: Create `mev/jit_arb.rs`**

```rust
use std::collections::HashMap;
use alloy::primitives::{Address, U256};
use crate::data::ExecutedLog;
use crate::pool::decoders::{decode_v3_mint_burn, decode_v3_swap, V3_SWAP_TOPIC, V3_MINT_TOPIC, V3_BURN_TOPIC};
use crate::pool::state::PoolManager;
use crate::mev::opportunity::MevOpportunity;
use crate::types::Strategy;

#[derive(Debug, Clone)]
struct SwapEvent {
    tx_index: usize,
    pool: Address,
    sender: Address,
}

#[derive(Debug, Clone)]
struct JitArbMint {
    mint_tx_index: usize,
    tick_lower: i32,
    tick_upper: i32,
    amount: u128,
    sender: Address,
    swapped: bool,
    burned: bool,
}

pub struct JitArbDetector {
    active_mints: HashMap<Address, Vec<JitArbMint>>,
    swap_events: Vec<SwapEvent>,
    emitted: Vec<(Address, usize, Address)>,
    block_number: u64,
}

impl JitArbDetector {
    pub fn new(block_number: u64) -> Self {
        JitArbDetector {
            active_mints: HashMap::new(),
            swap_events: Vec::new(),
            emitted: Vec::new(),
            block_number,
        }
    }

    pub fn process_tx(&mut self, tx_index: usize, logs: &[ExecutedLog], sender: Option<Address>) {
        let sender = match sender {
            Some(s) => s,
            None => return,
        };

        for log in logs {
            if log.topics.is_empty() {
                continue;
            }
            let t0 = log.topics[0];

            // Mint event
            if t0 == *V3_MINT_TOPIC {
                if let Some(decoded) = decode_v3_mint_burn(log) {
                    if decoded.amount > 0 {
                        self.active_mints
                            .entry(log.address)
                            .or_default()
                            .push(JitArbMint {
                                mint_tx_index: tx_index,
                                tick_lower: decoded.tick_lower,
                                tick_upper: decoded.tick_upper,
                                amount: decoded.amount as u128,
                                sender,
                                swapped: false,
                                burned: false,
                            });
                    }
                }
            }

            // Burn event
            if t0 == V3_BURN_TOPIC {
                if let Some(decoded) = decode_v3_mint_burn(log) {
                    if let Some(mints) = self.active_mints.get_mut(&log.address) {
                        for mint in mints.iter_mut() {
                            if mint.burned { continue; }
                            if mint.sender == sender
                                && mint.tick_lower == decoded.tick_lower
                                && mint.tick_upper == decoded.tick_upper
                                && mint.mint_tx_index <= tx_index
                            {
                                mint.burned = true;
                                break;
                            }
                        }
                    }
                }
            }

            // Swap event
            if t0 == V3_SWAP_TOPIC {
                self.swap_events.push(SwapEvent {
                    tx_index,
                    pool: log.address,
                    sender,
                });
                // Mark any matching active mints on this pool as swapped
                if let Some(mints) = self.active_mints.get_mut(&log.address) {
                    for mint in mints.iter_mut() {
                        if mint.sender == sender && mint.mint_tx_index <= tx_index {
                            mint.swapped = true;
                        }
                    }
                }
            }
        }
    }

    pub fn detect(&mut self, timestamp: u64, pm: &PoolManager) -> Vec<MevOpportunity> {
        let mut opportunities = Vec::new();
        let pool_addrs: Vec<Address> = self.active_mints.keys().copied().collect();

        for &pool_p in &pool_addrs {
            let Some(mints) = self.active_mints.get(&pool_p) else { continue };
            for mint in mints {
                let dedup_key = (pool_p, mint.mint_tx_index, mint.sender);
                if self.emitted.contains(&dedup_key) || !mint.swapped {
                    continue;
                }

                // Find swaps on pool_p by same sender
                let swaps_on_p: Vec<&SwapEvent> = self.swap_events.iter()
                    .filter(|s| s.pool == pool_p && s.sender == mint.sender && s.tx_index >= mint.mint_tx_index)
                    .collect();
                if swaps_on_p.is_empty() {
                    continue;
                }

                // Find swaps on a different pool Q sharing a token
                let mut found_arb = false;
                'outer: for swap_p in &swaps_on_p {
                    for swap_q in &self.swap_events {
                        if swap_q.pool == pool_p || swap_q.sender != mint.sender {
                            continue;
                        }
                        // Check proximity: within 1 tx index
                        let p_idx = swap_p.tx_index;
                        let q_idx = swap_q.tx_index;
                        let max_idx = p_idx.max(q_idx);
                        let min_idx = p_idx.min(q_idx);
                        if max_idx - min_idx > 1 {
                            continue;
                        }
                        // Check token sharing
                        if pools_share_token(pm, pool_p, swap_q.pool) {
                            self.emitted.push(dedup_key);
                            opportunities.push(Self::build_opp(
                                self.block_number, pool_p, swap_q.pool, mint, timestamp,
                            ));
                            found_arb = true;
                            break 'outer;
                        }
                    }
                    if found_arb { break; }
                }
            }
        }

        opportunities
    }

    fn build_opp(
        block_number: u64,
        jit_pool: Address,
        arb_pool: Address,
        mint: &JitArbMint,
        timestamp: u64,
    ) -> MevOpportunity {
        MevOpportunity {
            block_number,
            tx_index: mint.mint_tx_index,
            strategy: Strategy::JitArb,
            pool_a: jit_pool,
            pool_b: arb_pool,
            token_in: Address::ZERO,
            token_out: Address::ZERO,
            input_amount: U256::from(mint.amount),
            expected_profit: U256::ZERO,
            gas_cost_wei: 0,
            timestamp,
            path: Some(vec![jit_pool, arb_pool]),
            tick_lower: Some(mint.tick_lower),
            tick_upper: Some(mint.tick_upper),
            liquidity_amount: Some(mint.amount),
            victim_tx_index: None,
            backrun_tx_index: None,
        }
    }
}

fn pools_share_token(pm: &PoolManager, pool_a: Address, pool_b: Address) -> bool {
    let Some(info_a) = pm.get(pool_a).map(|p| p.info()) else { return false };
    let Some(info_b) = pm.get(pool_b).map(|p| p.info()) else { return false };
    info_a.token0 == info_b.token0
        || info_a.token0 == info_b.token1
        || info_a.token1 == info_b.token0
        || info_a.token1 == info_b.token1
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::{address, Bytes, B256};
    use crate::pool::state::{PoolManager, UniswapV2PoolState, PoolInfo, PoolState};

    fn pool_p() -> Address { address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa") }
    fn pool_q() -> Address { address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb") }
    fn wmatic() -> Address { address!("0d500b1d8e8ef31e21c99d1db9a6444d3adf1270") }
    fn usdc() -> Address { address!("2791bca1f2de4661ed88a30c99a7a9449aa84174") }
    fn sender() -> Address { address!("1111111111111111111111111111111111111111") }

    fn v3_mint_log(pool: Address, lower: i32, upper: i32, amount: u128) -> ExecutedLog {
        let mut data = Vec::new();
        let mut padded = [0u8; 32];
        padded[28..32].copy_from_slice(&lower.to_be_bytes());
        data.extend_from_slice(&padded);
        padded = [0u8; 32];
        padded[28..32].copy_from_slice(&upper.to_be_bytes());
        data.extend_from_slice(&padded);
        padded = [0u8; 32];
        padded[16..32].copy_from_slice(&amount.to_be_bytes());
        data.extend_from_slice(&padded);
        ExecutedLog { address: pool, topics: vec![*V3_MINT_TOPIC, B256::ZERO, B256::ZERO], data: data.into() }
    }

    fn v3_swap_log(pool: Address) -> ExecutedLog {
        ExecutedLog { address: pool, topics: vec![V3_SWAP_TOPIC, B256::ZERO, B256::ZERO], data: Bytes::from_static(&[0u8; 160]) }
    }

    fn make_pm() -> PoolManager {
        let mut pm = PoolManager::new();
        // Pool P: WMATIC/USDC
        pm.add_pool(PoolState::UniswapV2(UniswapV2PoolState {
            info: PoolInfo {
                address: pool_p(), token0: wmatic(), token1: usdc(), fee: 30, name: None,
                dex_type: crate::pool::dex_type::DexType::UniswapV2, tick_spacing: None,
            },
            reserve0: 1_000_000, reserve1: 1_000_000,
        }));
        // Pool Q: USDC/USDT (shares USDC with pool P)
        pm.add_pool(PoolState::UniswapV2(UniswapV2PoolState {
            info: PoolInfo {
                address: pool_q(), token0: usdc(), token1: address!("c2132d05d31c914a87c6611c10748aeb04b58e8f"),
                fee: 30, name: None,
                dex_type: crate::pool::dex_type::DexType::UniswapV2, tick_spacing: None,
            },
            reserve0: 1_000_000, reserve1: 1_000_000,
        }));
        pm
    }

    #[test]
    fn test_empty_detector_returns_nothing() {
        let mut detector = JitArbDetector::new(1);
        let pm = PoolManager::new();
        let opps = detector.detect(100, &pm);
        assert!(opps.is_empty());
    }

    #[test]
    fn test_mint_and_arb_same_tx() {
        let mut detector = JitArbDetector::new(1);
        let pm = make_pm();
        // Tx 0: Mint on pool P + swap on pool P + swap on pool Q by same sender
        detector.process_tx(0, &[
            v3_mint_log(pool_p(), -100, 100, 500_000),
            v3_swap_log(pool_p()),
            v3_swap_log(pool_q()),
        ], Some(sender()));
        let opps = detector.detect(100, &pm);
        assert_eq!(opps.len(), 1, "Same-tx Mint+arb should be detected");
        assert_eq!(opps[0].strategy, Strategy::JitArb);
        assert_eq!(opps[0].pool_a, pool_p());
        assert_eq!(opps[0].pool_b, pool_q());
        assert_eq!(opps[0].liquidity_amount, Some(500_000));
    }

    #[test]
    fn test_mint_then_arb_cross_tx() {
        let mut detector = JitArbDetector::new(1);
        let pm = make_pm();
        // Tx 0: Mint on pool P
        detector.process_tx(0, &[v3_mint_log(pool_p(), -100, 100, 500_000)], Some(sender()));
        assert!(detector.detect(100, &pm).is_empty(), "Mint alone should not trigger JitArb");
        // Tx 1: Swap on pool P + swap on pool Q
        detector.process_tx(1, &[v3_swap_log(pool_p()), v3_swap_log(pool_q())], Some(sender()));
        let opps = detector.detect(100, &pm);
        assert_eq!(opps.len(), 1, "Cross-tx Mint+arb should be detected");
    }

    #[test]
    fn test_different_sender_no_detection() {
        let mut detector = JitArbDetector::new(1);
        let pm = make_pm();
        let other = address!("2222222222222222222222222222222222222222");
        detector.process_tx(0, &[v3_mint_log(pool_p(), -100, 100, 500_000)], Some(sender()));
        detector.process_tx(1, &[v3_swap_log(pool_p()), v3_swap_log(pool_q())], Some(other));
        let opps = detector.detect(100, &pm);
        assert!(opps.is_empty(), "Different sender should not trigger JitArb");
    }

    #[test]
    fn test_no_token_share_no_detection() {
        let mut detector = JitArbDetector::new(1);
        // Pool R: DAI/LINK — no token shared with pool P (WMATIC/USDC)
        let pm = {
            let mut pm = make_pm();
            let pool_r = address!("cccccccccccccccccccccccccccccccccccccccc");
            pm.add_pool(PoolState::UniswapV2(UniswapV2PoolState {
                info: PoolInfo {
                    address: pool_r,
                    token0: address!("8f3cf7ad23cd3cadbd9735aff958023239c6a063"), // DAI
                    token1: address!("53e0bca35ec356bd5dddfebbd1fc0fd03fabad39"), // LINK
                    fee: 30, name: None,
                    dex_type: crate::pool::dex_type::DexType::UniswapV2, tick_spacing: None,
                },
                reserve0: 1_000_000, reserve1: 1_000_000,
            }));
            pm
        };
        detector.process_tx(0, &[
            v3_mint_log(pool_p(), -100, 100, 500_000),
            v3_swap_log(pool_p()),
            v3_swap_log(address!("cccccccccccccccccccccccccccccccccccccccc")),
        ], Some(sender()));
        let opps = detector.detect(100, &pm);
        assert!(opps.is_empty(), "No token sharing should not trigger JitArb");
    }

    #[test]
    fn test_no_duplicate_emission() {
        let mut detector = JitArbDetector::new(1);
        let pm = make_pm();
        detector.process_tx(0, &[
            v3_mint_log(pool_p(), -100, 100, 500_000),
            v3_swap_log(pool_p()),
            v3_swap_log(pool_q()),
        ], Some(sender()));
        assert_eq!(detector.detect(100, &pm).len(), 1);
        assert!(detector.detect(100, &pm).is_empty(), "Should not re-emit");
    }
}
```

- [ ] **Step 3: Run tests to verify they fail** (before adding module)

```bash
cd D:\gitlab.dte.repo\mev-bot-backtest
cargo test --lib mev::jit_arb::tests -- --nocapture 2>&1 || true
```
Expected: errors (module not found)

- [ ] **Step 4: Add module export and run tests again**

```bash
cargo test --lib mev::jit_arb::tests -- --nocapture
```
Expected: 6/6 pass

- [ ] **Step 5: Commit**

```bash
git add mev-backtest-core/src/mev/mod.rs mev-backtest-core/src/mev/jit_arb.rs
git commit -m "feat: add JitArbDetector for JIT+arbitrage combo detection"
```

---

### Task 2: Wire into run.rs

**Files:**
- Modify: `mev-backtest-core/src/run.rs`

- [ ] **Step 1: Add import after SandwichDetector import**

```rust
use crate::mev::jit_arb::JitArbDetector;
```

- [ ] **Step 2: Add initialization after SandwichDetector setup**

```rust
let mut jit_arb_detector = JitArbDetector::new(block_num);
```

- [ ] **Step 3: Add process_tx + detect calls after sandwich detection block**

```rust
// JitArb detector
let sender = *current_tx_from.borrow();
jit_arb_detector.process_tx(i, &tx.logs, sender);
let jit_arb_opps = jit_arb_detector.detect(timestamp, &*pool_manager.borrow());
if !jit_arb_opps.is_empty() {
    tracing::info!("Block {} tx {}: {} JitArb opportunities", block_num, i, jit_arb_opps.len());
}
all_opportunities.extend(jit_arb_opps);
```

- [ ] **Step 4: Verify compiles**

```bash
cargo check
```

- [ ] **Step 5: Commit**

```bash
git add mev-backtest-core/src/run.rs
git commit -m "feat: wire JitArbDetector into runner"
```

---

### Task 3: Integration test and verification

**Files:**
- Modify: `mev-backtest-core/tests/integration.rs`

- [ ] **Step 1: Add JitArb synthetic integration test**

After the sandwich test, add:

```rust
#[test]
fn test_jit_arb_detection_synthetic() {
    use mev_backtest_core::mev::jit_arb::JitArbDetector;
    use mev_backtest_core::pool::decoders::{V3_SWAP_TOPIC, V3_MINT_TOPIC};
    use mev_backtest_core::data::ExecutedLog;
    use alloy::primitives::{address, Address, Bytes, B256};

    let pool_p = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let pool_q = address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let sender = address!("1111111111111111111111111111111111111111");
    let wmatic = address!("0d500b1d8e8ef31e21c99d1db9a6444d3adf1270");
    let usdc = address!("2791bca1f2de4661ed88a30c99a7a9449aa84174");

    fn v3_mint_log(pool: Address, lower: i32, upper: i32, amount: u128) -> ExecutedLog {
        let mut data = Vec::new();
        let mut padded = [0u8; 32];
        padded[28..32].copy_from_slice(&lower.to_be_bytes());
        data.extend_from_slice(&padded);
        padded = [0u8; 32];
        padded[28..32].copy_from_slice(&upper.to_be_bytes());
        data.extend_from_slice(&padded);
        padded = [0u8; 32];
        padded[16..32].copy_from_slice(&amount.to_be_bytes());
        data.extend_from_slice(&padded);
        ExecutedLog { address: pool, topics: vec![*V3_MINT_TOPIC, B256::ZERO, B256::ZERO], data: data.into() }
    }

    fn v3_swap_log(pool: Address) -> ExecutedLog {
        ExecutedLog { address: pool, topics: vec![V3_SWAP_TOPIC, B256::ZERO, B256::ZERO], data: Bytes::from_static(&[0u8; 160]) }
    }

    // Build PoolManager with two pools sharing USDC
    let mut pm = mev_backtest_core::pool::state::PoolManager::new();
    pm.add_pool(mev_backtest_core::pool::state::PoolState::UniswapV2(
        mev_backtest_core::pool::state::UniswapV2PoolState {
            info: mev_backtest_core::pool::state::PoolInfo {
                address: pool_p, token0: wmatic, token1: usdc, fee: 30, name: None,
                dex_type: mev_backtest_core::pool::dex_type::DexType::UniswapV2, tick_spacing: None,
            },
            reserve0: 1_000_000, reserve1: 1_000_000,
        },
    ));
    pm.add_pool(mev_backtest_core::pool::state::PoolState::UniswapV2(
        mev_backtest_core::pool::state::UniswapV2PoolState {
            info: mev_backtest_core::pool::state::PoolInfo {
                address: pool_q,
                token0: usdc,
                token1: address!("c2132d05d31c914a87c6611c10748aeb04b58e8f"),
                fee: 30, name: None,
                dex_type: mev_backtest_core::pool::dex_type::DexType::UniswapV2, tick_spacing: None,
            },
            reserve0: 1_000_000, reserve1: 1_000_000,
        },
    ));

    let mut detector = JitArbDetector::new(42);
    detector.process_tx(0, &[
        v3_mint_log(pool_p, -100, 100, 500_000),
        v3_swap_log(pool_p),
        v3_swap_log(pool_q),
    ], Some(sender));

    let opps = detector.detect(12345, &pm);
    assert_eq!(opps.len(), 1, "Should detect JitArb");
    assert_eq!(opps[0].strategy, mev_backtest_core::types::Strategy::JitArb);
    assert_eq!(opps[0].pool_a, pool_p);
    assert_eq!(opps[0].pool_b, pool_q);
    assert_eq!(opps[0].liquidity_amount, Some(500_000));
    assert_eq!(opps[0].tick_lower, Some(-100));
    assert_eq!(opps[0].tick_upper, Some(100));
}
```

- [ ] **Step 2: Run all tests**

```bash
cargo test --lib
cargo test --test integration
cargo check --workspace
cargo clippy --all-targets 2>&1 || true
```

Expected:
- ~199 unit tests pass
- 19+ integration tests pass
- cargo check clean
- clippy clean (or only pre-existing)

- [ ] **Step 3: Commit**

```bash
git add mev-backtest-core/tests/integration.rs
git commit -m "test: add JitArb detection integration test"
```

---

### Task 4: Documentation in GUIDE.md

**Files:**
- Modify: `GUIDE.md`

- [ ] **Step 1: Read current GUIDE.md**

Read `GUIDE.md` to find insertion points.

- [ ] **Step 2: Add JitArb Detection section under Features**

After the Sandwich Detection section, add:

```markdown
### JIT Arbitrage (JitArb) Detection

The JitArb detector identifies a combination strategy where the same EOA:
1. Mints concentrated liquidity on a V3 pool (Mint)
2. Swaps through the JIT pool and a different pool sharing a token (Swap)
3. Captures both swap fees and arbitrage profit within the same block

This differs from standalone JIT liquidity detection: JitArb specifically requires cross-pool arbitrage trading by the liquidity deployer, not just any swap hitting the position.

**Pattern detected:**
- Mint on pool P + Swap on pool P + Swap on pool Q (tokens shared) by same sender

**Output fields:**
- `strategy`: `"jit_arb"`
- `pool_a`: The V3 pool where JIT was deployed
- `pool_b`: The other pool in the arbitrage
- `tick_lower`, `tick_upper`: The concentrated tick range
- `liquidity_amount`: Amount of liquidity deployed
- `path`: `[jit_pool, arb_pool]`

**Current limitations:**
- Expected profit and gas cost are not estimated (set to 0 in v1)
- Only V3 concentrated liquidity pools monitored
- Only detects 2-pool arb patterns (no multi-hop arb + JIT)
```

- [ ] **Step 3: Update strategy listing**

Find the strategy table line and update "implemented" list to include `jit_arb`:
```
`two_hop_arb`, `multi_hop_arb`, `jit`, and `jit_arb` are implemented. Only `sandwich` is parsed but not yet implemented.
```

- [ ] **Step 4: Commit**

```bash
git add GUIDE.md
git commit -m "docs: add JitArb detection section to GUIDE.md"
```

---

### Task 5: Final verification

- [ ] **Step 1: Run full test suite**

```bash
cargo test --lib
cargo test --test integration
cargo check --workspace
cargo clippy --all-targets 2>&1 || true
```

Expected: all tests pass, clippy clean.

- [ ] **Step 2: Show git status**

```bash
git status
git log --oneline -5
```

- [ ] **Step 3: If all good, final commit**

```bash
git add -A
git commit -m "feat: JIT arbitrage (JitArb) combo detection

- Add JitArbDetector detecting Mint + cross-pool arb by same sender
- Wire into runner alongside Jit and Sandwich detectors
- Add synthetic integration test
- Document in GUIDE.md"
```
