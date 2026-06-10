# Simulation Accuracy Design

## Overview

Address 6 accuracy issues in the backtest engine: gas estimation, USD pricing, sandwich profit, JIT profit, V3 per-tx state, and f64 precision. Delivered in 3 phases.

## Phase 1 Scope

### 1. Per-Strategy Gas Limits

Replace flat `gas_limit: u64 = 200_000` with strategy-specific defaults:

| Strategy   | Default gas | Rationale                     |
|------------|-------------|-------------------------------|
| TwoHopArb  | 150,000     | V2→V2: swap on each pool      |
| MultiHopArb| 300,000     | 3+ hops, more contract calls  |
| Jit        | 300,000     | Mint + Swap + Burn on V3      |
| JitArb     | 350,000     | Mint + Swap on P + Swap on Q  |
| Sandwich   | 200,000     | Frontrun + Backrun, same pool |

**Changes:**
- `GasConfig::compute_gas_cost()` takes a `Strategy` parameter
- `CliOverrides` supports optional per-strategy override map
- Config TOML gets optional `[gas_limits]` table
- All 5 detectors pass their strategy type when computing gas cost

**Files:** `types.rs`, `config.rs`, `mev/sandwich.rs`, `mev/two_hop.rs`, `mev/multi_hop.rs`, `mev/jit.rs`, `mev/jit_arb.rs`, `run.rs`

### 2. CoinGecko USD Pricing

New module `coingecko.rs` in core crate:

- Async HTTP client (`reqwest` added to core deps)
- `GET https://api.coingecko.com/api/v3/simple/price?id={asset_id}&vs_currencies=usd`
- Maps `ChainName` to CoinGecko asset IDs:
  - Polygon → `matic-network`
  - Ethereum → `ethereum`
  - BSC → `binancecoin`
  - Avalanche → `avalanche-2`
  - Arbitrum → `ethereum`
  - Base → `ethereum`
  - Optimism → `ethereum`
- In-memory cache with 5-minute TTL
- Graceful fallback: logs warning, uses last known price, or 0

**Config changes:**
- `Config.coingecko_api_key: Option<String>`
- `CliOverrides.coingecko_api_key: Option<String>`
- Validation: warn if missing for API/CLI report mode

**Integration:**
- Loaded lazily at aggregation/report time (not during detection)
- `aggregate()` takes optional `&PriceCache`
- CLI `report` command and API `map_opportunities` use it

**Files:** `coingecko.rs` (new), `Cargo.toml`, `config.rs`, `aggregate.rs`, `lib.rs`

### 3. Sandwich Profit Computation

**Current:** `expected_profit: U256::ZERO`

**Algorithm in `SandwichDetector::detect()`:**

```
frontrun = records[0]    // buys token → spends token_in
backrun  = records[2]    // sells token → receives token_in

// Gross profit in token_in units
profit_token_in = backrun.amount_out - frontrun.amount_in

// If token_in == wrapped_native (WMATIC/WETH/WBNB per chain config):
//   profit_wei = profit_token_in
// Else if token_out == wrapped_native:
//   profit_wei = profit_token_in * reserve_out / reserve_in (pool spot price)
// Else:
//   profit_wei = 0 (unstable coin pair, log warning)
```

**Config:** Add `wrapped_native_token: Address` per chain to `ChainConfig`

**Files:** `sandwich.rs`, `config.rs`, `pool/state.rs` (add helper to find pool by token pair)

### 4. f64 Precision Refinement

**Current:** All wei→ETH in aggregate.rs uses `wei as f64 / 1e18`

**Change:** Add `_wei` fields to metric structs:

```rust
pub struct StrategyMetrics {
    // existing f64 fields (for display)
    pub gross_revenue: f64,
    pub net_profit: f64,
    pub total_gas_cost: f64,
    // new wei-precise fields
    pub gross_revenue_wei: u128,
    pub net_profit_wei: i128,
    pub total_gas_cost_wei: u128,
}
```

Same for `SummaryMetrics` and `DexMetrics`. The `f64` fields remain for backward-compatible API output.

**Files:** `aggregate.rs`

## Phase 2 Scope (Future)

### JIT Fee Estimation
- Parse swap amount from V3 Swap events (`amount0`/`amount1` signed fields already decoded)
- Compute total fee: `amount_in * fee_tier / 10000`
- Approximate LP share: `mint_liquidity / (mint_liquidity + existing_liquidity_in_range)`
- Requires tick bitmap tracking for accurate range overlap

### V3 Per-Tx State
- Currently V3 swaps update `sqrt_price_x96`, `tick`, `liquidity` (done in `apply_v3_swap`)
- Issue: `ticks` map only contains positions modified by Mint/Burn events in the same block
- For full tick-crossing accuracy: track the full tick bitmap, or use a simplified constant-liquidity quote model

## Phase 3 Scope (Future)

### Exact Gas Simulation
- After detection, construct the MEV transaction and simulate via revm at the exact pre-tx state
- Use `CachedRpcDb` with current block's state root
- Capped to opportunities above a profit threshold

### Competition Modeling
- Configurable slippage tolerance parameter
- Reduces expected profit by X% to account for competing searchers

## Migration

All Phase 1 changes are backward-compatible:
- `GasConfig` default maintains identical behavior (uses per-strategy defaults)
- `aggregate()` f64 fields remain for API consumers
- `CoingeckoApiKey` defaults to `None` → skips USD price fetch
- Sandwich profit at 0 when conversion is ambiguous (same as current)
