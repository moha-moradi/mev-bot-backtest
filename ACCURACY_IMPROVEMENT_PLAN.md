# MEV Backtest — Accuracy & Efficiency Improvement Plan

**Priority**: Accuracy first, then efficiency.
**Constraint**: No external API-key-gated services. Prefer on-chain data.

---

## Phase 1: On-Chain USD Pricing (Highest Accuracy ROI)

**File**: `mev-backtest-core/src/mev/pricing.rs`

Replace hardcoded `OnceLock<HashMap>` with dynamic pricing derived from `PoolManager`:

```
onchain_usd_price(token, pm) → Option<f64>
```

Logic:
1. Find a WMATIC pair for `token` in PoolManager's token index
2. Compute `token_price_in_wmatic = reserve_wmatic / reserve_token`
3. Find WMATIC/USDC pool → compute `wmatic_usd_price = reserve_usdc / reserve_wmatic`
4. Return `token_price_in_wmatic * wmatic_usd_price`
5. Fallback to hardcoded constant if no pair found

**Zero extra RPC calls** — reserves already loaded. Block-accurate.

---

## Phase 2: Widen Tx Filter to Tokens (Medium Accuracy / Future-Proof)

**File**: `mev-backtest-core/src/run.rs`

Maintain `HashSet<Address>` of all unique token addresses across registered pools.

Change filter from pool-only to include tokens. Add `--fast-mode` flag that uses original narrow filter.

---

## Phase 3: Dynamic Gas Estimation (Medium Accuracy)

**File**: `mev-backtest-core/src/rpc.rs` + `mev-backtest-core/src/mev/two_hop.rs`

Add `estimate_gas(to, data)` to RpcClient. In TwoHopArbDetector, build calldata for the arb
path and call `estimate_gas` once per unique `(pool_a, pool_b, direction)` tuple.
Cache results in memory for the run. Fallback to 200k if estimation fails.

---

## Phase 4: Parallel Reserve Initialization (Efficiency)

**File**: `mev-backtest-core/src/pool/state.rs`

Replace sequential `for addr in pool_addrs` with `try_join_all` + semaphore (cap at 20).

---

## Phase 5: Precompute Arbitrage Pairs (Efficiency)

**File**: `mev-backtest-core/src/pool/state.rs`

Add cached pair list to `PoolManager` with dirty-flag invalidation on `add_pool`.

---

## Phase 6: Batch RPC eth_getProof (Efficiency)

**File**: `mev-backtest-core/src/replay.rs`

When 2+ storage slots from the same address are needed, batch via `eth_getProof`
instead of individual `eth_getStorageAt`.

---

## Phase 7: Cleanup Dead Code

**File**: `mev-backtest-core/src/fetch.rs`

Remove abandoned `handles` approach at lines 76-112.

---

## Phase 8: On-Chain Pool Discovery

**New file**: `mev-backtest-core/src/pool/discovery.rs`
**Modified**: `config.rs`, `pool/mod.rs`

Add `uniswap_v2_factories` to `ChainConfig`. Implement `PoolDiscoverer` that scans
`PairCreated` events from factory contracts, caches results in sled.
Merge with static JSON seed (dedup by address).

---

## Implementation Order

| Step | Description | Files |
|------|-------------|-------|
| 1 | On-chain USD pricing | `pricing.rs` |
| 2 | Widen tx filter + --fast-mode | `run.rs` |
| 3 | Dynamic gas estimation | `rpc.rs`, `two_hop.rs` |
| 4 | Parallel reserve init | `state.rs` |
| 5 | Precompute arb pairs | `state.rs` |
| 6 | Batch RPC eth_getProof | `replay.rs` |
| 7 | Cleanup dead code | `fetch.rs` |
| 8 | On-chain pool discovery | `discovery.rs`, `config.rs`, `mod.rs` |
