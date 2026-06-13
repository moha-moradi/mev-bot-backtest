# Cross-Verification Plan — Independent Second-Opinion Tests

## Goal

For each critical module in `core/src/`, write an **independent verification test** that cross-checks the tool's output against an alternative implementation or data source. This catches bugs that unit tests miss (wrong assumptions shared by both the code and its test).

---

## 1. Transaction Filter — Cross-Check Against `mevlog.rs`

**Module:** `replay.rs:replay_each_filtered()` (line 906)

**What it does:** During block replay, a filter decides which txs get full EVM execution vs. fast-path skip. It checks `tx.to` and log emitter addresses against tracked pool/token sets.

**Verification strategy:** For a sampled block, run `mevlog.rs` independently to classify every tx as "relevant" or "irrelevant". Compare the sets.

### Test: `test_filter_agrees_with_mevlog`

```rust
#[test]
fn test_filter_agrees_with_mevlog() {
    // Arrange: pick block N on Polygon with known DEX activity
    let block_num = 50_000_000u64;
    let (block, txs) = load_known_test_block(block_num);
    let receipts = load_known_test_receipts(block_num);
    let pool_addrs = load_tracked_pool_addresses();
    let token_addrs = load_tracked_token_addresses();

    // Act: run the mev-scout filter on each tx
    let mut scout_relevant = Vec::new();
    for (i, tx) in txs.iter().enumerate() {
        let receipt_logs = receipts.get(i).map(|r| r.logs.as_slice()).unwrap_or_default();
        let is_relevant = tx.to.is_some_and(|to| {
            pool_addrs.contains(&to) || token_addrs.contains(&to)
        }) || receipt_logs.iter().any(|l| {
            pool_addrs.contains(&l.address) || token_addrs.contains(&l.address)
        });
        if is_relevant {
            scout_relevant.push(i);
        }
    }

    // Act: run mevlog.rs (CLI subprocess or library call) on same block
    let mevlog_relevant = run_mevlog(block_num, &pool_addrs);

    // Assert: exact match on relevant tx indices
    assert_eq!(
        scout_relevant, mevlog_relevant,
        "Filter mismatch at block {}. Scout says {:#?}, mevlog says {:#?}",
        block_num, scout_relevant, mevlog_relevant
    );
}
```

**Test data:** Export a real block's txs + receipts as JSON fixtures (`tests/fixtures/block_50000000/`). Store in the repo.

**What it catches:** Filter false negatives (missed DEX interactions) and false positives (unnecessary EVM replay).

---

## 2. Pool State Machine — Reference Implementation

**Module:** `pool/state.rs:update_from_logs()` (line 599) + `apply_v2_swap/sync/v3_swap/mint_burn`

**What it does:** Maintains pool reserves by decoding Swap/Sync/Mint/Burn events from EVM logs during replay.

**Verification strategy:** Write a minimal reference state tracker in Rust (outside the main crate, or in `tests/`) that consumes the same event format and applies the same math. Feed both systems the same ordered event stream and compare state after each tx.

### Test structure

```
tests/reference_state.rs
```

```rust
mod reference {
    // Pure functions: given (reserve0, reserve1, amount0_in, amount1_in, amount0_out, amount1_out)
    // compute new reserves using constant-product formula
    pub fn apply_v2_swap(r0: u128, r1: u128, amt0_in: u128, amt1_in: u128, amt0_out: u128, amt1_out: u128) -> (u128, u128) {
        (r0.wrapping_add(amt0_in).wrapping_sub(amt0_out),
         r1.wrapping_add(amt1_in).wrapping_sub(amt1_out))
    }
}
```

```rust
#[test]
fn test_pool_state_against_reference_v2_swap() {
    // Feed the same Swap event to both PoolManager and reference
    // Compare reserve0/reserve1 after update
}
```

**Test data:** Serialize real event sequences from cache. Include edge cases:
- Swap with zero amounts
- Sync with both reserves at zero (new pool)
- Consecutive Swaps without intermediate Sync
- V3 Mint → Swap → Burn sequence

**What it catches:** Overflow/underflow bugs, wrong event field offsets, incorrect topic matching.

---

## 3. Arbitrage Quoting — Manual Constant-Product Calculator

**Module:** `mev/two_hop.rs:quote_path()` (line 109) + `pool/math.rs:optimal_two_hop_arb`

**What it does:** Computes optimal two-hop arbitrage profit between two pools sharing a token.

**Verification strategy:** For V2↔V2, the closed-form solution is deterministic. Implement a **brute-force search** over possible input amounts (0..max_input) in a separate test binary, find the maximum profit manually, and compare to the analytical result.

### Test: `test_arb_profit_matches_bruteforce`

```rust
#[test]
fn test_v2_arb_profit_matches_bruteforce() {
    // Fixed pool parameters
    let r1 = 1_000_000u128;  // pool A reserve of token1
    let r2 = 2_000_000;       // pool A reserve of shared token
    let r3 = 1_000_000;       // pool B reserve of shared token
    let r4 = 3_000_000;       // pool B reserve of token2
    let fee_a = 30u32;       // 0.3%
    let fee_b = 30u32;

    // Analytical result from optimal_two_hop_arb
    let analytical = optimal_two_hop_arb(r1, r2, fee_a, r3, r4, fee_b).unwrap();

    // Brute-force: try every possible input amount (step = 1)
    let mut best_profit = 0u128;
    let mut best_input = 0u128;
    for input in 0..=r1 {
        let mid = constant_product_output_amount(input, r1, r2, fee_a);
        if mid == 0 { continue; }
        let output = constant_product_output_amount(mid, r3, r4, fee_b);
        if output > input && output - input > best_profit {
            best_profit = output - input;
            best_input = input;
        }
    }

    assert_eq!(analytical.profit, best_profit,
        "Brute-force gives different profit. Analytical: input={}, profit={}. BF: input={}, profit={}",
        analytical.input_amount, analytical.profit, best_input, best_profit);
}
```

**For V3↔V3 and mixed pairs:** Use the same approach but with a coarser step (e.g., 1% of liquidity) since V3 quoting is more expensive. Tolerate 0.1% relative error due to tick approximation.

**What it catches:** Math errors in the closed-form solution (sign errors, fee handling, rounding direction).

---

## 4. Sandwich Detection — Known-Positive/Negative Datasets

**Module:** `mev/sandwich.rs:SandwichDetector` (line 135)

**What it does:** Detects front-run → victim → back-run swap triplets by the same EOA on the same pool.

**Verification strategy:** Build a dataset of **known sandwich blocks** (verified via Etherscan / EigenPhi / DexScreener) and **known clean blocks** (verified no sandwich).

### Test: `test_sandwich_detected_on_known_block`

```rust
#[test]
fn test_sandwich_detected_on_known_block() {
    // Block #XXXXXX on Polygon — manually verified sandwich on QuickSwap WMATIC/USDC
    let block_num = 49_000_000u64;
    let txs = load_known_sandwich_txs(block_num);
    let pm = load_known_sandwich_pools(block_num);

    let mut detector = SandwichDetector::new(block_num);
    // replay through txs in order, feeding logs to process_tx()
    for (i, tx) in txs.iter().enumerate() {
        detector.process_tx(i, &tx.logs, Some(tx.from));
    }
    let opps = detector.detect(tx_timestamp, &pm);

    assert_eq!(opps.len(), 1,
        "Expected 1 sandwich in known block {block_num}, got {}", opps.len());
    let opp = &opps[0];
    assert_eq!(opp.victim_tx_index, Some(known_victim_index));
    assert_eq!(opp.backrun_tx_index, Some(known_backrun_index));
    assert!(opp.expected_profit > U256::ZERO);
}
```

### Test: `test_no_false_positive_on_clean_block`

```rust
#[test]
fn test_no_false_positive_on_clean_block() {
    let block_num = 50_000_000u64; // known clean block
    // ... same setup, assert opps.is_empty()
}
```

**Test data fixtures:** Store minimal block data (only swap events + pool addresses needed, not full blocks). Keep under 50 KB each.

**What it catches:** False positives from spurious pattern matches, order-sensitive bugs, wrong `SwapDirection` detection.

---

## 5. Pool Address Registry — Canonical Source Cross-Check

**Module:** `config.rs:default_chains()` (line 166)

**What it does:** Hardcodes factory/router/pool addresses for each supported chain.

**Verification strategy:** Scrape the current addresses from the canonical source (DeFiLlama API, official DEX docs, or on-chain factory calls) and compare.

### Test: `test_contract_addresses_match_onchain`

```rust
#[tokio::test]
async fn test_polygon_quickswap_factory_matches_onchain() {
    let rpc = RpcClient::new("https://polygon-bor.publicnode.com", 137).unwrap();
    let cfg = Config::default();
    let polygon = cfg.chains.get("polygon").unwrap();

    if let Some(factories) = &polygon.uniswap_v2_factories {
        // QuickSwap factory at block 50,000,000
        let expected = &factories[0]; // "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32"
        let code = rpc.get_code(expected.parse().unwrap(), 50_000_000).await.unwrap();
        assert!(!code.is_empty(),
            "QuickSwap factory at {} has no code at block 50M — address may be wrong", expected);
    }
}
```

**Run for all 7 chains, all factory/vault/pool addresses.** Failures mean an address is outdated.

**What it catches:** Stale addresses when DEX contracts are upgraded or migrated.

---

## 6. Gas Cost — Empirical Validation Against On-Chain Data

**Module:** `types.rs:GasConfig::gas_limit_for_strategy()` (line 283)

**What it does:** Hardcodes per-strategy gas limits (150k for arb, 300k for JIT, etc.) used for profit estimation.

**Verification strategy:** For real MEV blocks, measure actual gas used by:
- Arbitrage bots: look up known MEV txs on Etherscan and record `gas_used`
- JIT providers: find Mint→Swap→Burn txs by the same sender
- Sandwich bots: find front-run/back-run txs

### Test: `test_gas_limits_are_reasonable`

```rust
#[test]
fn test_arb_gas_limit_is_not_too_low() {
    // Known arbitrage tx on Ethereum, block 18,000,000
    let actual_gas = 142_000u64; // from receipt
    let our_limit = GasConfig::default().gas_limit_for_strategy(
        Strategy::TwoHopArb, &HashMap::new());
    assert!(our_limit >= actual_gas,
        "Gas limit {} too low for tx using {} gas", our_limit, actual_gas);
    assert!(our_limit <= actual_gas * 3,
        "Gas limit {} is unreasonably high (tx used {})", our_limit, actual_gas);
}
```

Populate a table of 10-20 known MEV txs per strategy type from historical blocks.

**What it catches:** Gas estimates that are too low (false negative on profitable txs) or too high (overestimate costs, false negative on marginal txs).

---

## 7. Implementation Order

| # | Module | Test file | Dependencies |
|---|--------|-----------|--------------|
| 1 | Pool state reference | `tests/reference_state.rs` | None, pure math |
| 2 | Two-hop arb brute-force | `tests/arb_bruteforce.rs` | `pool/math.rs` |
| 3 | Transaction filter vs mevlog | `tests/mevlog_crosscheck.rs` | Block fixtures, `mevlog` binary |
| 4 | Sandwich known datasets | `tests/known_sandwiches.rs` | Block fixtures |
| 5 | Address registry on-chain | `tests/address_registry.rs` | RPC endpoint |
| 6 | Gas cost empirical | `tests/gas_empirical.rs` | Hardcoded tx data |

---

## 8. Test Data Fixtures

Store under `core/tests/fixtures/`:

```
tests/fixtures/
├── block_50000000/
│   ├── block.json          # BlockData
│   ├── txs.json            # Vec<TxData>
│   └── receipts.json       # Vec<ReceiptData>
├── block_49000000/         # known sandwich block
│   ├── txs.json
│   └── pools.json          # PoolState data (reserves before block)
├── known_arb_txs.csv       # tx_hash, block, gas_used, strategy
└── README.md               # source provenance (block explorer links)
```

Each fixture is a small JSON file (< 1 MB). Generate them with a helper script:

```
cargo run --bin export-fixtures -- --block 50000000 --chain polygon --rpc <URL>
```

---

## 9. What This Covers

| Module | Verification | Catches |
|--------|-------------|---------|
| `replay.rs` filter | Second tool (mevlog.rs) | Silent false negatives |
| `pool/state.rs` | Reference state machine | Arithmetic bugs, decoding errors |
| `mev/two_hop.rs` | Brute-force search | Closed-form math errors |
| `mev/sandwich.rs` | Known real blocks | Pattern match false +/- |
| `config.rs` | On-chain RPC calls | Stale contract addresses |
| `types.rs` gas | Empirical tx data | Unrealistic cost estimates |
