# Sandwich Detection — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Detect sandwich attacks on Uniswap V2 pools — where a searcher frontruns a user's swap, the user trades at a worse price, and the searcher backruns to capture the profit — all within the same block.

**Architecture:** A stateful `SandwichDetector` that accumulates V2 Swap event records across sequential transactions in a block (sliding window of `SwapRecord`), then scans for the frontrun→victim→backrun pattern on the same pool where frontrun and backrun share the same sender EOA. Uses `PoolManager` in `detect()` to resolve token addresses for output opportunities.

**Tech Stack:** Rust, alloy (U256/Address/B256), V2 swap event decoder (topic `0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822`), existing `PoolManager` for pool info lookup.

---

### Design: SandwichDetector Sliding Window

`SandwichDetector` maintains per-block state:

```
swap_records: Vec<SwapRecord>   — all V2 swap events seen so far in this block
emitted: Vec<(Address, usize)>  — dedup by (pool, frontrun_tx_index)
```

Each `SwapRecord`:
- `tx_index: usize` — transaction index
- `sender: Address` — EOA that sent the tx (from `TxData.from`)
- `pool: Address` — pool contract address
- `direction: SwapDirection` — `Token0ForToken1` or `Token1ForToken0`
- `amount_in: u128` — input token amount
- `amount_out: u128` — output token amount

Per tx in `process_tx()`:
1. Scan all logs for V2 Swap events (`topic0 == V2_SWAP_TOPIC`)
2. Decode `amount0In, amount1In, amount0Out, amount1Out` from data (128 bytes)
3. Determine direction and amounts
4. Push `SwapRecord` to `swap_records`

`SwapDirection` derivation:
- `amount0In > 0 && amount1Out > 0` → `Token0ForToken1`, `amount_in = amount0In`, `amount_out = amount1Out`
- `amount1In > 0 && amount0Out > 0` → `Token1ForToken0`, `amount_in = amount1In`, `amount_out = amount0Out`

In `detect()`:
1. Group `swap_records` by `pool`
2. For each pool's records (ordered by `tx_index`), examine consecutive triples via `.windows(3)`
3. Triple `[a, b, c]` matches if:
   - `a.sender == c.sender` — same EOA (searcher frontruns and backruns)
   - `a.direction == b.direction` — victim swaps same direction as frontrun
   - `a.direction != c.direction` — backrun reverses direction
   - Not already emitted
4. Build `MevOpportunity` referencing PoolManager for `token_in`/`token_out`

```
Example (USDC/WMATIC pool, token0=USDC, token1=WMATIC):
  Tx 0: Searcher buys WMATIC (Token0ForToken1)  — frontrun
  Tx 1: User buys WMATIC (Token0ForToken1)       — victim (worse price)
  Tx 2: Searcher sells WMATIC (Token1ForToken0) — backrun (profit)
```

---

### Task 1: Add sandwich-specific fields to MevOpportunity

**Files:**
- Modify: `mev-backtest-core/src/mev/opportunity.rs`
- Test: unit tests in opportunity.rs

- [ ] **Step 1: Add `victim_tx_index` and `backrun_tx_index` optional fields**

After `pub liquidity_amount: Option<u128>,` (line 42), add:

```rust
    /// Transaction index of the victim transaction (sandwich detection)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub victim_tx_index: Option<usize>,
    /// Transaction index of the backrun transaction (sandwich detection)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backrun_tx_index: Option<usize>,
```

The existing `tx_index` field stores the frontrun tx index. These two additional fields capture the full sandwich triple.

- [ ] **Step 2: Update the `test_mev_opportunity_jit_fields_roundtrip` test to include the new fields (set to None)**

Append after the existing JIT test:

```rust
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
        expected_profit: U256::from(100),
        gas_cost_wei: 1_000_000,
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
    assert!(!json_no.contains("backrun_tx_index"));
}
```

- [ ] **Step 3: Verify compiles and test passes**

```bash
cargo test --lib mev::opportunity::tests -- --nocapture
```

- [ ] **Step 4: Commit**

```bash
git add mev-backtest-core/src/mev/opportunity.rs
git commit -m "feat: add sandwich fields (victim_tx_index, backrun_tx_index) to MevOpportunity"
```

---

### Task 2: Create `mev/sandwich.rs` — SandwichDetector

**Files:**
- Create: `mev-backtest-core/src/mev/sandwich.rs`
- Modify: `mev-backtest-core/src/mev/mod.rs`
- Test: unit tests within sandwich.rs

- [ ] **Step 1: Add module export to mev/mod.rs**

Add after `pub mod jit;`:
```rust
pub mod sandwich;
```

- [ ] **Step 2: Create `mev/sandwich.rs`**

```rust
use std::collections::HashMap;
use alloy::primitives::{b256, Address, B256, U256};
use crate::data::ExecutedLog;
use crate::mev::opportunity::MevOpportunity;
use crate::pool::state::PoolManager;
use crate::types::Strategy;

/// Uniswap V2 Swap event topic:
/// Swap(address indexed sender, uint256 amount0In, uint256 amount1In, uint256 amount0Out, uint256 amount1Out, address indexed to)
const V2_SWAP_TOPIC: B256 =
    b256!("d78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822");

/// Direction of a swap on a pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SwapDirection {
    Token0ForToken1,
    Token1ForToken0,
}

/// A decoded V2 swap event recorded during process_tx().
#[derive(Debug, Clone)]
struct SwapRecord {
    tx_index: usize,
    sender: Address,
    pool: Address,
    direction: SwapDirection,
    amount_in: u128,
    amount_out: u128,
}

/// Detects sandwich attacks on Uniswap V2 pools.
///
/// Stateful per block: accumulates V2 Swap event records across sequential txs.
/// After each tx in block order, call `process_tx()` then `detect()`.
///
/// Pattern detected:
/// - **Frontrun (tx N):** Searcher swaps on pool P, moving price
/// - **Victim (tx N+1):** User swaps on pool P at worse price (same direction)
/// - **Backrun (tx N+2):** Searcher reverses position on pool P at profit (opposite direction)
///
/// Requires frontrun and backrun to come from the same EOA.
pub struct SandwichDetector {
    /// All swap records accumulated this block, in tx order
    swap_records: Vec<SwapRecord>,
    /// Dedup by (pool, frontrun_tx_index)
    emitted: Vec<(Address, usize)>,
    /// Current block number
    block_number: u64,
}

impl SandwichDetector {
    pub fn new(block_number: u64) -> Self {
        SandwichDetector {
            swap_records: Vec::new(),
            emitted: Vec::new(),
            block_number,
        }
    }

    /// Process a single transaction's logs and sender address.
    /// Decodes V2 Swap events and records them.
    /// Call BEFORE `detect()` for each tx in block order.
    pub fn process_tx(
        &mut self,
        tx_index: usize,
        logs: &[ExecutedLog],
        sender: Option<Address>,
    ) {
        let Some(sender) = sender else { return };

        for log in logs {
            if log.topics.is_empty() || log.topics[0] != V2_SWAP_TOPIC {
                continue;
            }
            if log.data.len() < 128 {
                continue;
            }

            let amt0_in = u128_from_be_bytes_32(&log.data[..32]);
            let amt1_in = u128_from_be_bytes_32(&log.data[32..64]);
            let amt0_out = u128_from_be_bytes_32(&log.data[64..96]);
            let amt1_out = u128_from_be_bytes_32(&log.data[96..128]);

            let (direction, amount_in, amount_out) =
                if amt0_in > 0 && amt1_out > 0 {
                    (SwapDirection::Token0ForToken1, amt0_in, amt1_out)
                } else if amt1_in > 0 && amt0_out > 0 {
                    (SwapDirection::Token1ForToken0, amt1_in, amt0_out)
                } else {
                    continue; // unknown or invalid direction
                };

            self.swap_records.push(SwapRecord {
                tx_index,
                sender,
                pool: log.address,
                direction,
                amount_in,
                amount_out,
            });
        }
    }

    /// Returns new sandwich opportunities detected since the last call.
    /// Call AFTER `process_tx()` for each tx. Requires PoolManager for token resolution.
    pub fn detect(
        &mut self,
        timestamp: u64,
        pool_manager: &PoolManager,
    ) -> Vec<MevOpportunity> {
        let mut opportunities = Vec::new();

        // Group records by pool
        let mut pool_records: HashMap<Address, Vec<&SwapRecord>> = HashMap::new();
        for record in &self.swap_records {
            pool_records.entry(record.pool).or_default().push(record);
        }

        // Scan each pool's records for sandwich triples
        for (_pool, records) in &pool_records {
            // records are in tx_index order (pushed in order during process_tx)
            for window in records.windows(3) {
                let front = &window[0];
                let victim = &window[1];
                let back = &window[2];

                // Check dedup
                let dedup_key = (front.pool, front.tx_index);
                if self.emitted.contains(&dedup_key) {
                    continue;
                }

                // Sandwich pattern:
                // 1. Frontrun and backrun from same EOA
                // 2. Victim swaps same direction as frontrun
                // 3. Backrun reverses direction
                if front.sender != back.sender {
                    continue;
                }
                if front.direction != victim.direction {
                    continue;
                }
                if front.direction == back.direction {
                    continue;
                }

                self.emitted.push(dedup_key);

                let pool_info = pool_manager.get(&front.pool)
                    .map(|p| p.info())
                    .ok_or(())
                    .ok();

                let (token_in, token_out) = match pool_info {
                    Some(info) => match front.direction {
                        SwapDirection::Token0ForToken1 => (info.token0, info.token1),
                        SwapDirection::Token1ForToken0 => (info.token1, info.token0),
                    },
                    None => (Address::ZERO, Address::ZERO),
                };

                opportunities.push(MevOpportunity {
                    block_number: self.block_number,
                    tx_index: front.tx_index,
                    strategy: Strategy::Sandwich,
                    pool_a: front.pool,
                    pool_b: Address::ZERO,
                    token_in,
                    token_out,
                    input_amount: U256::from(front.amount_in),
                    expected_profit: U256::ZERO,
                    gas_cost_wei: 0,
                    timestamp,
                    path: None,
                    tick_lower: None,
                    tick_upper: None,
                    liquidity_amount: None,
                    victim_tx_index: Some(victim.tx_index),
                    backrun_tx_index: Some(back.tx_index),
                });
            }
        }

        opportunities
    }
}

/// Decode a uint128 from the last 16 bytes of a 32-byte slice.
fn u128_from_be_bytes_32(bytes: &[u8]) -> u128 {
    let start = bytes.len().saturating_sub(16);
    let mut buf = [0u8; 16];
    buf.copy_from_slice(&bytes[start..start + 16]);
    u128::from_be_bytes(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::{address, Bytes, B256};
    use crate::data::ExecutedLog;
    use crate::pool::state::{PoolInfo, PoolState, UniswapV2PoolState};
    use crate::pool::dex_type::DexType;

    fn encode_u256(val: u128) -> Vec<u8> {
        let mut buf = vec![0u8; 16];
        buf.extend_from_slice(&val.to_be_bytes());
        buf
    }

    fn v2_swap_log(pool: Address, amt0_in: u128, amt1_in: u128, amt0_out: u128, amt1_out: u128) -> ExecutedLog {
        let mut data = Vec::with_capacity(128);
        data.extend_from_slice(&encode_u256(amt0_in));
        data.extend_from_slice(&encode_u256(amt1_in));
        data.extend_from_slice(&encode_u256(amt0_out));
        data.extend_from_slice(&encode_u256(amt1_out));
        ExecutedLog {
            address: pool,
            topics: vec![V2_SWAP_TOPIC, B256::ZERO, B256::ZERO],
            data: data.into(),
        }
    }

    fn pool_a() -> Address { address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa") }
    fn pool_b() -> Address { address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb") }
    fn alice() -> Address { address!("1111111111111111111111111111111111111111") }
    fn bob() -> Address { address!("2222222222222222222222222222222222222222") }

    fn make_pm_with_pool(pool_addr: Address, t0: Address, t1: Address) -> PoolManager {
        let mut pm = PoolManager::new();
        pm.add_pool(PoolState::UniswapV2(UniswapV2PoolState {
            info: PoolInfo {
                address: pool_addr,
                token0: t0,
                token1: t1,
                fee: 30,
                name: None,
                dex_type: DexType::UniswapV2,
                tick_spacing: None,
            },
            reserve0: 1_000_000,
            reserve1: 1_000_000,
        }));
        pm
    }

    #[test]
    fn test_empty_detector_returns_nothing() {
        let mut detector = SandwichDetector::new(1);
        let pm = PoolManager::new();
        let opps = detector.detect(100, &pm);
        assert!(opps.is_empty());
    }

    #[test]
    fn test_sandwich_detected() {
        let mut detector = SandwichDetector::new(1);
        let pm = make_pm_with_pool(pool_a(), address!("cccccccccccccccccccccccccccccccccccccccc"), address!("dddddddddddddddddddddddddddddddddddddddd"));

        // Tx 0: Frontrun — alice sells token0 for token1 (Token0ForToken1)
        detector.process_tx(0, &[v2_swap_log(pool_a(), 100, 0, 0, 90)], Some(alice()));
        assert!(detector.detect(100, &pm).is_empty(), "Single swap is not a sandwich");

        // Tx 1: Victim — sells token0 for token1 at worse price
        detector.process_tx(1, &[v2_swap_log(pool_a(), 200, 0, 0, 170)], Some(bob()));
        assert!(detector.detect(100, &pm).is_empty(), "Two swaps not a sandwich");

        // Tx 2: Backrun — alice sells token1 for token0 (reverse direction)
        detector.process_tx(2, &[v2_swap_log(pool_a(), 0, 85, 105, 0)], Some(alice()));
        let opps = detector.detect(100, &pm);
        assert_eq!(opps.len(), 1, "Three swaps with same EOA front/back should be a sandwich");

        let opp = &opps[0];
        assert_eq!(opp.strategy, Strategy::Sandwich);
        assert_eq!(opp.pool_a, pool_a());
        assert_eq!(opp.tx_index, 0);
        assert_eq!(opp.victim_tx_index, Some(1));
        assert_eq!(opp.backrun_tx_index, Some(2));
        assert_ne!(opp.token_in, Address::ZERO);
        assert_ne!(opp.token_out, Address::ZERO);
    }

    #[test]
    fn test_different_eoa_no_sandwich() {
        let mut detector = SandwichDetector::new(1);
        let pm = PoolManager::new();

        // Three swaps on same pool, but all from different EOAs
        detector.process_tx(0, &[v2_swap_log(pool_a(), 100, 0, 0, 90)], Some(alice()));
        detector.process_tx(1, &[v2_swap_log(pool_a(), 200, 0, 0, 170)], Some(bob()));
        detector.process_tx(2, &[v2_swap_log(pool_a(), 0, 85, 105, 0)], Some(address!("3333333333333333333333333333333333333333")));

        let opps = detector.detect(100, &pm);
        assert!(opps.is_empty(), "Different front/back EOAs should not match");
    }

    #[test]
    fn test_same_direction_backrun_no_sandwich() {
        let mut detector = SandwichDetector::new(1);
        let pm = PoolManager::new();

        // Frontrun and backrun same direction (not a reversal)
        detector.process_tx(0, &[v2_swap_log(pool_a(), 100, 0, 0, 90)], Some(alice()));
        detector.process_tx(1, &[v2_swap_log(pool_a(), 200, 0, 0, 170)], Some(bob()));
        detector.process_tx(2, &[v2_swap_log(pool_a(), 300, 0, 0, 250)], Some(alice()));

        let opps = detector.detect(100, &pm);
        assert!(opps.is_empty(), "Backrun same direction as frontrun should not match");
    }

    #[test]
    fn test_no_duplicate_emission() {
        let mut detector = SandwichDetector::new(1);
        let pm = PoolManager::new();

        detector.process_tx(0, &[v2_swap_log(pool_a(), 100, 0, 0, 90)], Some(alice()));
        detector.process_tx(1, &[v2_swap_log(pool_a(), 200, 0, 0, 170)], Some(bob()));
        detector.process_tx(2, &[v2_swap_log(pool_a(), 0, 85, 105, 0)], Some(alice()));

        let opps = detector.detect(100, &pm);
        assert_eq!(opps.len(), 1);

        // Second detect should return nothing
        let opps2 = detector.detect(100, &pm);
        assert!(opps2.is_empty(), "Should not re-emit same opportunity");
    }

    #[test]
    fn test_multiple_pools_independent() {
        let mut detector = SandwichDetector::new(1);
        let pm = PoolManager::new();

        // Pool A: complete sandwich
        detector.process_tx(0, &[v2_swap_log(pool_a(), 100, 0, 0, 90)], Some(alice()));
        detector.process_tx(1, &[v2_swap_log(pool_a(), 200, 0, 0, 170)], Some(bob()));
        detector.process_tx(2, &[v2_swap_log(pool_a(), 0, 85, 105, 0)], Some(alice()));

        // Pool B: only two swaps
        detector.process_tx(3, &[v2_swap_log(pool_b(), 50, 0, 0, 45)], Some(alice()));
        detector.process_tx(4, &[v2_swap_log(pool_b(), 100, 0, 0, 85)], Some(bob()));

        let opps = detector.detect(100, &pm);
        assert_eq!(opps.len(), 1, "Only pool A should trigger a sandwich");
        assert_eq!(opps[0].pool_a, pool_a());
    }

    #[test]
    fn test_single_tx_no_detection() {
        let mut detector = SandwichDetector::new(1);
        let pm = PoolManager::new();
        detector.process_tx(0, &[v2_swap_log(pool_a(), 100, 0, 0, 90)], Some(alice()));
        let opps = detector.detect(100, &pm);
        assert!(opps.is_empty());
    }

    #[test]
    fn test_two_txs_no_detection() {
        let mut detector = SandwichDetector::new(1);
        let pm = PoolManager::new();
        detector.process_tx(0, &[v2_swap_log(pool_a(), 100, 0, 0, 90)], Some(alice()));
        detector.process_tx(1, &[v2_swap_log(pool_a(), 200, 0, 0, 170)], Some(bob()));
        let opps = detector.detect(100, &pm);
        assert!(opps.is_empty());
    }

    #[test]
    fn test_interleaved_pool_swaps_no_false_positive() {
        let mut detector = SandwichDetector::new(1);
        let pm = PoolManager::new();

        // tx 0: pool A frontrun (alice)
        detector.process_tx(0, &[v2_swap_log(pool_a(), 100, 0, 0, 90)], Some(alice()));
        // tx 1: pool B swap (alice) — not a victim for pool A
        detector.process_tx(1, &[v2_swap_log(pool_b(), 50, 0, 0, 45)], Some(bob()));
        // tx 2: pool A backrun (alice) — but no victim on pool A in between
        detector.process_tx(2, &[v2_swap_log(pool_a(), 0, 85, 105, 0)], Some(alice()));

        // Pool A records: tx 0 and tx 2 — only 2 records, no window of 3
        // Pool B records: tx 1 — only 1 record
        let opps = detector.detect(100, &pm);
        assert!(opps.is_empty(), "No victim on pool A means no sandwich");
    }
}
```

- [ ] **Step 3: Verify compiles and tests pass**

```bash
cargo test --lib mev::sandwich::tests -- --nocapture
cargo check
```

- [ ] **Step 4: Commit**

```bash
git add mev-backtest-core/src/mev/mod.rs \
      mev-backtest-core/src/mev/sandwich.rs
git commit -m "feat: add SandwichDetector for V2 sandwich attack detection"
```

---

### Task 3: Wire SandwichDetector into run.rs

**Files:**
- Modify: `mev-backtest-core/src/run.rs`

Same pattern as JitDetector — `SandwichDetector` is stateful per-block, needs `TxData.from` bridged via `RefCell`.

- [ ] **Step 1: Add import**

After `use crate::mev::jit::JitDetector;`:
```rust
use crate::mev::sandwich::SandwichDetector;
```

- [ ] **Step 2: Initialize SandwichDetector alongside JitDetector**

In `run_block()`, replace:
```rust
let mut jit_detector = JitDetector::new(block_num);
```
with:
```rust
let mut jit_detector = JitDetector::new(block_num);
let mut sandwich_detector = SandwichDetector::new(block_num);
```

- [ ] **Step 3: Add SandwichDetector calls in the on_tx callback**

After the JIT detector block (after `all_opportunities.extend(jit_opps);`), add:

```rust
// Sandwich detector
sandwich_detector.process_tx(i, &tx.logs, sender);
let sandwich_opps = sandwich_detector.detect(timestamp, &pm);
if !sandwich_opps.is_empty() {
    tracing::info!(
        "Block {} tx {}: {} sandwich opportunities",
        block_num,
        i,
        sandwich_opps.len()
    );
}
all_opportunities.extend(sandwich_opps);
```

- [ ] **Step 4: Verify compiles**

```bash
cargo check
```

- [ ] **Step 5: Commit**

```bash
git add mev-backtest-core/src/run.rs mev-backtest-core/src/mev/sandwich.rs
git commit -m "feat: wire SandwichDetector into runner"
```

---

### Task 4: Integration tests

**Files:**
- Modify: `mev-backtest-core/tests/integration.rs`

- [ ] **Step 1: Add synthetic sandwich detection integration test**

Before the real-data tests section, add:

```rust
#[test]
fn test_sandwich_detection_synthetic() {
    use mev_backtest_core::mev::sandwich::SandwichDetector;
    use mev_backtest_core::data::ExecutedLog;
    use alloy::primitives::{address, B256, Bytes};

    let pool = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let alice = address!("1111111111111111111111111111111111111111");
    let bob = address!("2222222222222222222222222222222222222222");

    // V2 Swap topic hash
    let v2_swap_topic: B256 =
        b256!("d78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822");

    fn v2_swap_log(pool: Address, amt0_in: u128, amt1_in: u128, amt0_out: u128, amt1_out: u128) -> ExecutedLog {
        let mut data = Vec::with_capacity(128);
        let mut buf = vec![0u8; 16];
        data.extend_from_slice(&buf);
        data.extend_from_slice(&amt0_in.to_be_bytes());
        buf = vec![0u8; 16];
        data.extend_from_slice(&buf);
        data.extend_from_slice(&amt1_in.to_be_bytes());
        buf = vec![0u8; 16];
        data.extend_from_slice(&buf);
        data.extend_from_slice(&amt0_out.to_be_bytes());
        buf = vec![0u8; 16];
        data.extend_from_slice(&buf);
        data.extend_from_slice(&amt1_out.to_be_bytes());
        ExecutedLog { address: pool, topics: vec![v2_swap_topic, B256::ZERO, B256::ZERO], data: data.into() }
    }

    let mut pm = mev_backtest_core::pool::state::PoolManager::new();
    let usdc = address!("2791bca1f2de4661ed88a30c99a7a9449aa84174");
    let wmatic = address!("0d500b1d8e8ef31e21c99d1db9a6444d3adf1270");
    pm.add_pool(mev_backtest_core::pool::state::PoolState::UniswapV2(
        mev_backtest_core::pool::state::UniswapV2PoolState {
            info: mev_backtest_core::pool::state::PoolInfo {
                address: pool,
                token0: usdc,
                token1: wmatic,
                fee: 30,
                name: None,
                dex_type: mev_backtest_core::pool::dex_type::DexType::UniswapV2,
                tick_spacing: None,
            },
            reserve0: 1_000_000,
            reserve1: 1_000_000,
        }
    ));

    let mut detector = SandwichDetector::new(42);
    let timestamp = 12345u64;

    // Tx 0: alice frontruns — buys WMATIC (token0→token1)
    detector.process_tx(0, &[v2_swap_log(pool, 100, 0, 0, 90)], Some(alice));
    assert!(detector.detect(timestamp, &pm).is_empty());

    // Tx 1: bob (victim) — buys WMATIC at worse price
    detector.process_tx(1, &[v2_swap_log(pool, 200, 0, 0, 170)], Some(bob));
    assert!(detector.detect(timestamp, &pm).is_empty());

    // Tx 2: alice backruns — sells WMATIC (token1→token0)
    detector.process_tx(2, &[v2_swap_log(pool, 0, 85, 105, 0)], Some(alice));
    let opps = detector.detect(timestamp, &pm);
    assert!(!opps.is_empty(), "Should detect sandwich");
    assert_eq!(opps[0].strategy, mev_backtest_core::types::Strategy::Sandwich);
    assert_eq!(opps[0].pool_a, pool);
    assert_eq!(opps[0].victim_tx_index, Some(1));
    assert_eq!(opps[0].backrun_tx_index, Some(2));
    assert_eq!(opps[0].token_in, usdc);
    assert_eq!(opps[0].token_out, wmatic);

    // No duplicate
    assert!(detector.detect(timestamp, &pm).is_empty());
}
```

- [ ] **Step 2: Add import to integration.rs**

At the top, add alongside existing imports:
```rust
use alloy::primitives::b256;
use mev_backtest_core::mev::sandwich::SandwichDetector;
```

- [ ] **Step 3: Run tests**

```bash
cargo test --lib mev::sandwich::tests -- --nocapture
cargo test --test integration -- --nocapture
```

- [ ] **Step 4: Commit**

```bash
git add mev-backtest-core/tests/integration.rs
git commit -m "test: add sandwich detection integration tests"
```

---

### Task 5: Documentation in GUIDE.md

**Files:**
- Modify: `GUIDE.md`

- [ ] **Step 1: Read current GUIDE.md to find insertion points**

```bash
# GUIDE.md is 642 lines, read relevant sections
```

- [ ] **Step 2: Add Sandwich Detection section under Features**

After the JIT Liquidity Detection section (around line 465), add:

```markdown
### Sandwich Detection

The sandwich detector identifies frontrunning attacks on Uniswap V2 pools where a searcher exploits a user's pending swap. The pattern spans three consecutive (or nearby) transactions interacting with the same pool:

1. **Frontrun (tx N):** The searcher buys/sells tokens on pool P, moving the price.
2. **Victim (tx N+1):** The user's swap executes on pool P at the worsened price.
3. **Backrun (tx N+2):** The searcher reverses their position on pool P at a profit.

All three transactions must interact with the same pool. The frontrun and backrun must come from the same EOA (the searcher).

**Pattern matched:**
- Same pool for all three transactions
- Frontrun and backrun share the same sender address
- Victim swaps in the same direction as the frontrun
- Backrun swaps in the opposite direction (reversal)
- Sliding window over swap records grouped by pool

**Output fields:**
- `strategy`: `"sandwich"`
- `pool_a`: The V2 pool where the sandwich occurred
- `tx_index`: Transaction index of the frontrun
- `victim_tx_index`: Transaction index of the victim
- `backrun_tx_index`: Transaction index of the backrun
- `token_in`, `token_out`: Tokens involved (resolved from pool metadata)

Note: Sandwich detection is always active when running backtests. No separate CLI flag is needed.

**Current limitations:**
- Only Uniswap V2 pools are monitored (V3 support planned)
- Expected profit and gas cost are not estimated (set to 0 in v1)
- Only detects strict consecutive triples on the same pool (no gap handling)
- Does not verify actual price impact — relies on direction matching
```

- [ ] **Step 3: Commit**

```bash
git add GUIDE.md
git commit -m "docs: add sandwich detection section to GUIDE.md"
```

---

### Task 6: Final verification

- [ ] **Step 1: Run full test suite**

```bash
cargo test --lib
cargo test --test integration
cargo check --workspace
```

Expected: all existing tests pass, new SandwichDetector unit tests pass. All 5 strategies have detection paths (two_hop_arb, multi_hop_arb, jit, sandwich), though jit_arb is still a placeholder.

- [ ] **Step 2: Run clippy**

```bash
cargo clippy --all-targets 2>&1 || true
```

- [ ] **Step 3: Show git status**

```bash
git status
git log --oneline -5
```

- [ ] **Step 4: If all good, commit any remaining changes**

```bash
git add -A
git commit -m "feat: complete sandwich detection on Uniswap V2

- Add victim_tx_index and backrun_tx_index to MevOpportunity
- Implement SandwichDetector with sliding window of SwapRecords
- Wire into runner with TxData sender bridge
- Add synthetic integration test
- Document sandwich detection in GUIDE.md"
```
