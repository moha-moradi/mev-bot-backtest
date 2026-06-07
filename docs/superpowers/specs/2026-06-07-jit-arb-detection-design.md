# JitArb Detection Design

## Overview

Detect **JIT liquidity + arbitrage combo** (JitArb): an LP deploys concentrated liquidity on a V3 pool P, then executes an arbitrage trade that swaps through pool P (benefiting from their own concentrated liquidity) and one or more other pools — capturing both swap fees and arbitrage profit.

## Detection Algorithm

### Core Signal

Same EOA in same block:
1. **V3 Mint** on pool P (deploy concentrated liquidity in a tick range)
2. **V3 Swap** on pool P (trades against the JIT position)
3. **V3 Swap** on a *different* pool Q (completes the arb)

Conditions:
- Mint tx_index ≤ Swap tx_index (causal order)
- Same sender (tx.from) for Mint and Swaps
- Pool P and Pool Q share at least one token
- All events in the same block

### Pseudocode

```
process_tx(tx_index, logs, tx_sender):
  for each log:
    if V3 Mint (amount > 0):
      active_mints[log.address].push({ tx_index, tick_lower, tick_upper, amount, sender: tx_sender })
    if V3 Burn:
      mark matching (pool, tick_range, sender) as burned
    if V3 Swap:
      swap_events.push({ tx_index, pool: log.address, sender: tx_sender })

detect(timestamp, pool_manager):
  for each (pool_p, mints) in active_mints:
    for each mint in mints:
      if !mint.swapped || already_emitted(pool_p, mint.mint_tx_index, mint.sender): continue
      // Find swaps on pool_p by same sender
      let swaps_on_p = swap_events where pool == pool_p, sender == mint.sender, tx_index >= mint.tx_index
      if swaps_on_p.empty: continue
      // Find swaps on other pools by same sender at nearby tx_index
      for each swap_p in swaps_on_p:
        for each swap_q in swap_events where pool != pool_p, sender == mint.sender:
          if |swap_q.tx_index - swap_p.tx_index| <= 1 AND pools_share_token(pool_manager, pool_p, pool_q):
            emit JitArb(pool_p, pool_q, mint)
            mark_emitted(pool_p, mint.mint_tx_index, mint.sender)
```

### pools_share_token

Uses `PoolManager` (same pattern as SandwichDetector):

```
pools_share_token(pm, pool_a, pool_b):
  info_a = pm.get(pool_a).info()
  info_b = pm.get(pool_b).info()
  return info_a.token0 == info_b.token0
      || info_a.token0 == info_b.token1
      || info_a.token1 == info_b.token0
      || info_a.token1 == info_b.token1
```

## Data Structures

### SwapEvent

```rust
struct SwapEvent {
    tx_index: usize,
    pool: Address,
    sender: Address,
}
```

Lightweight record of a V3 Swap event, grouped per tx in `process_tx`.

`swap_events` is **not cleared** between `detect()` calls — it accumulates across the block. This enables cross-tx detection (Swap on P in tx1, Swap on Q in tx2 — `detect` sees both at step 2). Deduplication via `emitted` prevents re-emission.

### JitArbMint

```rust
struct JitArbMint {
    mint_tx_index: usize,
    tick_lower: i32,
    tick_upper: i32,
    amount: u128,
    sender: Address,
    swapped: bool,
    burned: bool,
}
```

Same fields as `ActiveMint` in `JitDetector`, but `sender` is non-optional (`Address` not `Option<Address>`).

### JitArbDetector

```rust
pub struct JitArbDetector {
    active_mints: HashMap<Address, Vec<JitArbMint>>,
    swap_events: Vec<SwapEvent>,
    emitted: Vec<(Address, usize, Address)>,  // (pool, mint_tx_index, sender)
    block_number: u64,
}
```

### MevOpportunity (no new fields)

```rust
MevOpportunity {
    strategy: Strategy::JitArb,
    pool_a: jit_pool,       // pool where JIT was deployed
    pool_b: arb_pool,        // the other pool in the arb
    token_in: Address::ZERO, // v1: not computed
    token_out: Address::ZERO,
    input_amount: U256::from(mint.amount),
    expected_profit: U256::ZERO,  // v1: deferred
    gas_cost_wei: 0,
    tick_lower: Some(mint.tick_lower),
    tick_upper: Some(mint.tick_upper),
    liquidity_amount: Some(mint.amount),
    path: Some(vec![jit_pool, arb_pool]),
    victim_tx_index: None,
    backrun_tx_index: None,
}
```

All required fields already exist in `MevOpportunity`.

## Files

| File | Action |
|------|--------|
| `src/mev/jit_arb.rs` | **NEW** — JitArbDetector (~200 lines) |
| `src/mev/mod.rs` | Add `pub mod jit_arb;` |
| `src/run.rs` | Wire in JitArbDetector per block |

## Integration in run.rs

```rust
// Per block:
let mut jit_arb_detector = JitArbDetector::new(block_num);

// In on_tx closure, after sandwich detection:
jit_arb_detector.process_tx(i, &tx.logs, sender);
let jit_arb_opps = jit_arb_detector.detect(timestamp, &pm); // &pm is PoolManager
all_opportunities.extend(jit_arb_opps);
```

Follows the same pattern as `SandwichDetector::detect(timestamp, &pm)`.

## Edge Cases

| Case | Handling |
|------|----------|
| Same-tx Mint + arb | `process_tx` processes Mints before swaps → state is ready |
| 3+ pool arb | Fires if any swap hits a pool sharing a token with JIT pool |
| Multiple concurrent JIT positions | Independent per (pool, mint_tx_index, sender) |
| V2 pools | Ignored — only V3 Mint/Burn logs are checked |
| No burn | Still detected as JitArb (burn is optional) |

## Differences from JitDetector

| Aspect | JitDetector | JitArbDetector |
|--------|------------|----------------|
| Detection | Mint → Swap (any swap on pool) | Mint → Swap → cross-pool swap by same sender |
| Swap tracking | Marks mint.swapped = true | Collects SwapEvent pool+sender+tx_index |
| emit condition | Any swap on pool | Swap on pool + cross-pool swap sharing token |
| PoolManager needed | No | Yes (token sharing check) |

## Tasks

1. **Create `jit_arb.rs`** — struct, process_tx() for Mint/Burn/Swap events
2. **Implement `detect()`** — cross-pool matching, emit dedup
3. **Wire into `run.rs`** — instantiation, process_tx + detect calls
4. **Unit tests** — same-tx JitArb, cross-tx, no false positives
5. **Build verification** — `cargo build && cargo test`

## Blockers / Dependencies

- Event decoders already exist (`decode_v3_mint_burn`, `V3_SWAP_TOPIC`)
- PoolManager token queries available
- `Strategy::JitArb` already defined in types.rs
- No new dependencies required
