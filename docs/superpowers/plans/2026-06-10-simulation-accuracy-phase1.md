# Simulation Accuracy — Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 4 accuracy issues: per-strategy gas limits, CoinGecko USD pricing, sandwich profit computation, f64 precision

**Architecture:** Pure additive changes — no refactoring. All new code lives in `coingecko.rs`. Gas and profit changes are minimal mutations to existing functions. USD prices are fetched lazily at aggregation time, keeping the detection pipeline pure.

**Tech Stack:** Rust, reqwest (new dep), CoinGecko API, existing PoolManager for sandwich price conversion

---

### Task 1: Add reqwest dependency and coingecko module scaffold

**Files:**
- Modify: `mev-backtest-core/Cargo.toml`
- Modify: `mev-backtest-core/src/lib.rs`
- Create: `mev-backtest-core/src/coingecko.rs`
- Create: `mev-backtest-core/src/aggregate.rs` (read first to plan)

- [ ] **Step 1: Add reqwest to Cargo.toml**

```toml
# Add after revm line
reqwest = { version = "0.12", features = ["json"] }
```

- [ ] **Step 2: Add module declaration to lib.rs and read aggregate.rs**

Insert in `lib.rs` (alphabetical order after `cache`):
```rust
pub mod coingecko;
```

Then read `aggregate.rs`:
```bash
Get-Content -Path "mev-backtest-core/src/aggregate.rs" | Select-Object -First 20
```

- [ ] **Step 3: Write the PriceCache stub and a failing test**

In `coingecko.rs`:
```rust
//! CoinGecko USD pricing with caching.
//!
//! Provides live USD exchange rates for native tokens of supported chains.
//! Prices are fetched once and cached in-memory with a configurable TTL.

use crate::types::ChainName;

/// Cached USD price for a chain's native token.
#[derive(Debug, Clone)]
pub struct PriceEntry {
    pub usd: f64,
    pub fetched_at: std::time::Instant,
}

/// In-memory price cache with TTL.
#[derive(Debug)]
pub struct PriceCache {
    entry: Option<PriceEntry>,
    ttl: std::time::Duration,
    api_key: Option<String>,
}

impl PriceCache {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            entry: None,
            ttl: std::time::Duration::from_secs(300), // 5 minutes
            api_key,
        }
    }

    /// Get USD price for a chain's native token.
    /// Returns cached value if fresh, otherwise fetches from API.
    pub async fn usd_price(&mut self, chain: ChainName) -> Option<f64> {
        // Stub: always returns None for now
        let _ = chain;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_cache_returns_none_when_empty() {
        let mut cache = PriceCache::new(None);
        let price = futures::executor::block_on(cache.usd_price(ChainName::Polygon));
        assert!(price.is_none());
    }
}
```

- [ ] **Step 4: Run test to verify it fails initially (module not recognized)**

```bash
cargo test -p mev-backtest-core --test '*' -- coingecko 2>&1 | Select-Object -First 20
```

Expected: Tests not found (module not compiled yet since we only wrote the file)

- [ ] **Step 5: Run all existing tests to confirm no regressions**

```bash
cargo test -p mev-backtest-core 2>&1
```

Expected: All tests pass (or known failures)

- [ ] **Step 6: Commit**

```bash
git add mev-backtest-core/Cargo.toml mev-backtest-core/src/lib.rs mev-backtest-core/src/coingecko.rs
git commit -m "feat(core): add coingecko module scaffold with PriceCache"
```

---

### Task 2: Per-strategy gas limits in GasConfig

**Files:**
- Modify: `mev-backtest-core/src/types.rs`

- [ ] **Step 1: Read the current GasConfig**

```bash
Get-Content -Path "mev-backtest-core/src/types.rs" | Select-String -Pattern "GasConfig" -Context 0,30
```

- [ ] **Step 2: Add `gas_limit_for_strategy` method to GasConfig**

Replace the existing `compute_gas_cost` method:

```rust
impl GasConfig {
    /// Strategy-specific default gas limits based on empirical observations.
    pub fn gas_limit_for_strategy(&self, strategy: Strategy) -> u64 {
        match strategy {
            Strategy::TwoHopArb => 150_000,
            Strategy::MultiHopArb => 300_000,
            Strategy::Jit => 300_000,
            Strategy::JitArb => 350_000,
            Strategy::Sandwich => 200_000,
        }
    }

    pub fn compute_gas_cost(&self, strategy: Strategy, base_fee_per_gas: u128) -> u128 {
        let gas_limit = self.gas_limit_for_strategy(strategy);
        let pf_wei = (self.priority_fee_gwei * 1_000_000_000.0) as u128;
        let effective_price = match self.gas_model {
            GasModel::HistoricalExact => base_fee_per_gas.saturating_add(pf_wei),
            GasModel::Fixed => pf_wei,
            GasModel::P90 => base_fee_per_gas.saturating_mul(150).saturating_div(100).saturating_add(pf_wei),
        };
        (gas_limit as u128).saturating_mul(effective_price)
    }
}
```

- [ ] **Step 3: Update the existing gas tests**

Replace the existing test functions in the `#[cfg(test)]` block of `types.rs`:

```rust
#[test]
fn test_gas_config_default_compute_historical_exact() {
    let cfg = GasConfig::default();
    let cost = cfg.compute_gas_cost(Strategy::TwoHopArb, 50_000_000_000);
    assert_eq!(cost, 150_000u128 * 50_000_000_000);
}

#[test]
fn test_gas_config_priority_fee() {
    let cfg = GasConfig {
        priority_fee_gwei: 2.0,
        ..GasConfig::default()
    };
    let cost = cfg.compute_gas_cost(Strategy::TwoHopArb, 50_000_000_000u128);
    assert_eq!(cost, 150_000u128 * 52_000_000_000u128);
}

#[test]
fn test_gas_config_fixed_model() {
    let cfg = GasConfig {
        gas_model: GasModel::Fixed,
        priority_fee_gwei: 3.0,
        ..GasConfig::default()
    };
    let cost = cfg.compute_gas_cost(Strategy::TwoHopArb, 50_000_000_000u128);
    assert_eq!(cost, 150_000u128 * 3_000_000_000u128);
}

#[test]
fn test_gas_config_p90_model() {
    let cfg = GasConfig {
        gas_model: GasModel::P90,
        priority_fee_gwei: 1.0,
        ..GasConfig::default()
    };
    let cost = cfg.compute_gas_cost(Strategy::TwoHopArb, 50_000_000_000u128);
    assert_eq!(cost, 150_000u128 * 76_000_000_000u128);
}

#[test]
fn test_gas_limit_per_strategy() {
    let cfg = GasConfig::default();
    assert_eq!(cfg.gas_limit_for_strategy(Strategy::TwoHopArb), 150_000);
    assert_eq!(cfg.gas_limit_for_strategy(Strategy::MultiHopArb), 300_000);
    assert_eq!(cfg.gas_limit_for_strategy(Strategy::Jit), 300_000);
    assert_eq!(cfg.gas_limit_for_strategy(Strategy::JitArb), 350_000);
    assert_eq!(cfg.gas_limit_for_strategy(Strategy::Sandwich), 200_000);
}
```

- [ ] **Step 4: Run gas tests**

```bash
cargo test -p mev-backtest-core types::tests 2>&1
```

Expected: All pass

- [ ] **Step 5: Run all existing tests — expect compile errors (call sites need updating)**

```bash
cargo test -p mev-backtest-core 2>&1 | Select-Object -First 40
```

Expected: Compile errors at all `compute_gas_cost` call sites (wrong number of arguments)

- [ ] **Step 6: Commit**

```bash
git add mev-backtest-core/src/types.rs
git commit -m "feat(core): per-strategy gas limits in GasConfig"
```

---

### Task 3: Update all call sites for strategy-aware gas cost

**Files:**
- Modify: `mev-backtest-core/src/mev/two_hop.rs`
- Modify: `mev-backtest-core/src/mev/multi_hop.rs`
- Modify: `mev-backtest-core/src/mev/jit.rs`
- Modify: `mev-backtest-core/src/mev/jit_arb.rs`
- Modify: `mev-backtest-core/src/mev/sandwich.rs`
- Modify: `mev-backtest-core/src/run.rs`

- [ ] **Step 1: Update two_hop.rs**

Find the `compute_gas_cost` call (pass `Strategy::TwoHopArb`):
```rust
// Change:
let gas_cost_wei = gas_config.compute_gas_cost(base_fee_per_gas);
// To:
let gas_cost_wei = gas_config.compute_gas_cost(Strategy::TwoHopArb, base_fee_per_gas);
```

Verify the strategy import exists; if not, add `use crate::types::Strategy;` at the top.

- [ ] **Step 2: Update multi_hop.rs**

Find `compute_gas_cost` call, change to pass `Strategy::MultiHopArb`:
```rust
let gas_cost_wei = gas_config.compute_gas_cost(Strategy::MultiHopArb, base_fee_per_gas);
```

- [ ] **Step 3: Update jit.rs**

Find `compute_gas_cost` call, change to pass `Strategy::Jit`:
```rust
let gas_cost_wei = gas_config.compute_gas_cost(Strategy::Jit, base_fee_per_gas);
```

- [ ] **Step 4: Update jit_arb.rs**

Find `compute_gas_cost` call, change to pass `Strategy::JitArb`:
```rust
let gas_cost_wei = gas_config.compute_gas_cost(Strategy::JitArb, base_fee_per_gas);
```

- [ ] **Step 5: Update sandwich.rs**

Find `compute_gas_cost` call, change to pass `Strategy::Sandwich`:
```rust
let gas_cost_wei = gas_config.compute_gas_cost(Strategy::Sandwich, base_fee_per_gas);
```

- [ ] **Step 6: Update run.rs**

Find any `compute_gas_cost` calls in `run.rs`. Search with:
```bash
Get-Content -Path "mev-backtest-core/src/run.rs" | Select-String -Pattern "compute_gas_cost"
```

Update each call site with the appropriate strategy. If used generically, choose `Strategy::TwoHopArb` as default or pass through from the calling context.

- [ ] **Step 7: Run all tests to confirm compilation and no regressions**

```bash
cargo test -p mev-backtest-core 2>&1
```

Expected: All tests compile and pass

- [ ] **Step 8: Commit**

```bash
git add mev-backtest-core/src/mev/two_hop.rs mev-backtest-core/src/mev/multi_hop.rs mev-backtest-core/src/mev/jit.rs mev-backtest-core/src/mev/jit_arb.rs mev-backtest-core/src/mev/sandwich.rs mev-backtest-core/src/run.rs
git commit -m "feat(core): pass strategy to compute_gas_cost at all call sites"
```

---

### Task 4: Write the full CoinGecko client

**Files:**
- Modify: `mev-backtest-core/src/coingecko.rs`

- [ ] **Step 1: Read the current stub to plan**

```bash
Get-Content -Path "mev-backtest-core/src/coingecko.rs"
```

- [ ] **Step 2: Implement the full CoinGecko client**

Replace the stub in `coingecko.rs`:

```rust
//! CoinGecko USD pricing with caching.
//!
//! Provides live USD exchange rates for native tokens of supported chains.
//! Prices are fetched once and cached in-memory with a configurable TTL.

use crate::types::ChainName;

/// Maps our ChainName to CoinGecko's asset identifier.
fn coingecko_asset_id(chain: ChainName) -> &'static str {
    match chain {
        ChainName::Polygon => "matic-network",
        ChainName::Ethereum => "ethereum",
        ChainName::Bsc => "binancecoin",
        ChainName::Avalanche => "avalanche-2",
        ChainName::Arbitrum => "ethereum",
        ChainName::Base => "ethereum",
        ChainName::Optimism => "ethereum",
    }
}

/// Cached USD price for a chain's native token.
#[derive(Debug, Clone)]
pub struct PriceEntry {
    pub usd: f64,
    pub fetched_at: std::time::Instant,
}

/// In-memory price cache with TTL.
#[derive(Debug)]
pub struct PriceCache {
    // Key = coingecko asset id, value = price entry
    entries: std::collections::HashMap<String, PriceEntry>,
    ttl: std::time::Duration,
    api_key: Option<String>,
}

/// Response shape from CoinGecko `/simple/price`.
#[derive(serde::Deserialize)]
struct CoinGeckoPriceResponse {
    #[serde(default)]
    usd: f64,
}

impl PriceCache {
    /// Create a new price cache with the given optional API key.
    ///
    /// Free tier (no API key) works but has rate limits of 10-30 req/min.
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            entries: std::collections::HashMap::new(),
            ttl: std::time::Duration::from_secs(300),
            api_key,
        }
    }

    /// Set a custom TTL for cached prices.
    pub fn with_ttl(mut self, ttl: std::time::Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Get USD price for a chain's native token.
    /// Returns cached value if fresh, otherwise fetches from API.
    pub async fn usd_price(&mut self, chain: ChainName) -> Option<f64> {
        let asset_id = coingecko_asset_id(chain);

        // Check cache
        if let Some(entry) = self.entries.get(asset_id) {
            if entry.fetched_at.elapsed() < self.ttl {
                return Some(entry.usd);
            }
        }

        // Fetch from API
        match self.fetch_price(asset_id).await {
            Ok(usd) => {
                self.entries.insert(asset_id.to_string(), PriceEntry {
                    usd,
                    fetched_at: std::time::Instant::now(),
                });
                Some(usd)
            }
            Err(e) => {
                // Fall back to stale cache if available
                if let Some(entry) = self.entries.get(asset_id) {
                    tracing::warn!("CoinGecko fetch failed, using stale price: {e}");
                    return Some(entry.usd);
                }
                tracing::warn!("CoinGecko fetch failed and no cached price: {e}");
                None
            }
        }
    }

    /// Execute the HTTP request to CoinGecko.
    async fn fetch_price(&self, asset_id: &str) -> Result<f64, anyhow::Error> {
        let url = format!(
            "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd",
            asset_id
        );

        let client = reqwest::Client::new();
        let mut req = client.get(&url);

        // Add API key header if provided (CoinGecko Pro/Demo tier)
        if let Some(key) = &self.api_key {
            req = req.header("x-cg-demo-api-key", key);
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("CoinGecko returned HTTP {}", resp.status());
        }

        // Response is like: {"ethereum":{"usd":3500.0}}
        let map: std::collections::HashMap<String, CoinGeckoPriceResponse> = resp.json().await?;
        match map.get(asset_id) {
            Some(entry) => Ok(entry.usd),
            None => anyhow::bail!("asset '{asset_id}' not found in CoinGecko response"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coingecko_asset_id_mapping() {
        assert_eq!(coingecko_asset_id(ChainName::Polygon), "matic-network");
        assert_eq!(coingecko_asset_id(ChainName::Ethereum), "ethereum");
        assert_eq!(coingecko_asset_id(ChainName::Bsc), "binancecoin");
        assert_eq!(coingecko_asset_id(ChainName::Avalanche), "avalanche-2");
        assert_eq!(coingecko_asset_id(ChainName::Arbitrum), "ethereum");
        assert_eq!(coingecko_asset_id(ChainName::Base), "ethereum");
        assert_eq!(coingecko_asset_id(ChainName::Optimism), "ethereum");
    }

    #[test]
    fn test_price_cache_starts_empty() {
        let cache = PriceCache::new(None);
        assert!(cache.entries.is_empty());
    }

    #[test]
    fn test_price_cache_with_ttl() {
        let cache = PriceCache::new(None).with_ttl(std::time::Duration::from_secs(60));
        assert_eq!(cache.ttl.as_secs(), 60);
    }
}
```

- [ ] **Step 3: Run the new tests**

```bash
cargo test -p mev-backtest-core coingecko 2>&1
```

Expected: All tests pass

- [ ] **Step 4: Run full test suite**

```bash
cargo test -p mev-backtest-core 2>&1
```

Expected: All pass

- [ ] **Step 5: Commit**

```bash
git add mev-backtest-core/src/coingecko.rs
git commit -m "feat(core): full CoinGecko client with caching"
```

---

### Task 5: Add config fields for CoinGecko, wrapped native token, and gas overrides

**Files:**
- Modify: `mev-backtest-core/src/config.rs`
- Modify: `mev-backtest-core/src/types.rs` (add wrapped native to ChainConfig concept)

- [ ] **Step 1: Read config.rs to plan**

```bash
Get-Content -Path "mev-backtest-core/src/config.rs" | Select-String -Pattern "pub struct Config" -Context 0,50
```

- [ ] **Step 2: Add fields to Config struct**

In `config.rs`, add these fields to the `Config` struct:

```rust
    /// CoinGecko API key for USD price lookups. Optional — free tier works but is rate-limited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coingecko_api_key: Option<String>,
    /// Optional per-strategy gas limit overrides.
    /// Keys are strategy names like "two_hop_arb", "sandwich", etc.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub gas_limits: std::collections::HashMap<String, u64>,
```

- [ ] **Step 3: Add field to ChainConfig**

Add to `ChainConfig`:

```rust
    /// Address of the chain's wrapped native token (e.g., WMATIC on Polygon, WETH on Ethereum)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrapped_native_token: Option<String>,
```

- [ ] **Step 4: Update default_chains() with wrapped native addresses**

In `default_chains()`, for each chain, add the wrapped native token:

```rust
// Polygon
ChainConfig {
    wrapped_native_token: Some("0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270".to_string()),
    ..polygon_rest
}
// Ethereum
ChainConfig {
    wrapped_native_token: Some("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string()),
    ..ethereum_rest
}
// BSC
ChainConfig {
    wrapped_native_token: Some("0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c".to_string()),
    ..bsc_rest
}
// Avalanche
ChainConfig {
    wrapped_native_token: Some("0xB31f66AA3C1e785363F0875A1B74E27b85FD66c7".to_string()),
    ..avalanche_rest
}
// Arbitrum
ChainConfig {
    wrapped_native_token: Some("0x82aF49447D8a07e3bd95BD0d56f35241523fBab1".to_string()),
    ..arbitrum_rest
}
// Base
ChainConfig {
    wrapped_native_token: Some("0x4200000000000000000000000000000000000006".to_string()),
    ..base_rest
}
// Optimism
ChainConfig {
    wrapped_native_token: Some("0x4200000000000000000000000000000000000006".to_string()),
    ..optimism_rest
}
```

- [ ] **Step 5: Add fields to CliOverrides**

```rust
    pub coingecko_api_key: Option<String>,
```

- [ ] **Step 6: Update merge_cli**

```rust
        if let Some(v) = &overrides.coingecko_api_key {
            self.coingecko_api_key = Some(v.clone());
        }
```

- [ ] **Step 7: Add method to resolve effective gas limit with overrides**

In `impl GasConfig` in `types.rs`, update `gas_limit_for_strategy` to accept an optional overrides map:

```rust
    pub fn gas_limit_for_strategy(
        &self,
        strategy: Strategy,
        overrides: &std::collections::HashMap<String, u64>,
    ) -> u64 {
        let key = strategy.to_string();
        if let Some(&limit) = overrides.get(&key) {
            return limit;
        }
        match strategy {
            Strategy::TwoHopArb => 150_000,
            Strategy::MultiHopArb => 300_000,
            Strategy::Jit => 300_000,
            Strategy::JitArb => 350_000,
            Strategy::Sandwich => 200_000,
        }
    }
```

And update `compute_gas_cost` to accept the overrides:

```rust
    pub fn compute_gas_cost(
        &self,
        strategy: Strategy,
        base_fee_per_gas: u128,
        overrides: &std::collections::HashMap<String, u64>,
    ) -> u128 {
        let gas_limit = self.gas_limit_for_strategy(strategy, overrides);
        // ... rest same as before
    }
```

- [ ] **Step 8: Update all call sites to pass `&HashMap::new()` or the real overrides**

Update each `compute_gas_cost` call in detectors to add the third parameter. For now, pass `&std::collections::HashMap::new()`:

```rust
// In each detector:
let gas_cost_wei = gas_config.compute_gas_cost(Strategy::Xxx, base_fee_per_gas, &HashMap::new());
```

- [ ] **Step 9: Update test compute_gas_cost calls**

```rust
let cost = cfg.compute_gas_cost(Strategy::TwoHopArb, 50_000_000_000, &HashMap::new());
let cost = cfg.compute_gas_cost(Strategy::TwoHopArb, 50_000_000_000u128, &HashMap::new());
// etc.
```

- [ ] **Step 10: Run tests**

```bash
cargo test -p mev-backtest-core 2>&1
```

Expected: All pass

- [ ] **Step 11: Commit**

```bash
git add mev-backtest-core/src/config.rs mev-backtest-core/src/types.rs
git commit -m "feat(core): add coingecko_api_key, wrapped_native_token, gas_limits to config"
```

---

### Task 6: Compute sandwich profit in sandwich.rs

**Files:**
- Modify: `mev-backtest-core/src/mev/sandwich.rs`
- Modify: `mev-backtest-core/src/pool/state.rs` (add helper)

- [ ] **Step 1: Read sandwich.rs detect() method**

```bash
Get-Content -Path "mev-backtest-core/src/mev/sandwich.rs" | Select-Object -First 100
```

- [ ] **Step 2: Add `wrapped_native` field to PoolManager and accessor**

In `pool/state.rs`, find the `PoolManager` struct definition (search `pub struct PoolManager`) and add the field:

```rust
pub struct PoolManager {
    pub pools: HashMap<Address, PoolState>,
    pub infos: HashMap<Address, PoolInfo>,
    pub token_index: HashMap<Address, HashSet<Address>>,
    pub chain_name: Option<ChainName>,
    /// Address of the wrapped native token (WMATIC/WETH/WBNB) per chain.
    pub wrapped_native: Option<Address>,
}
```

Then add the accessor method:

```rust
impl PoolManager {
    /// Check if the given address is the wrapped native token (e.g., WMATIC, WETH).
    pub fn is_wrapped_native(&self, token: &Address) -> bool {
        self.wrapped_native.as_ref().map_or(false, |wn| token == wn)
    }

    /// Get V2 pool state by address (returns None if not a V2 pool or not found).
    pub fn get_v2_state(&self, address: &Address) -> Option<&UniswapV2PoolState> {
        match self.pools.get(address) {
            Some(PoolState::UniswapV2(state)) => Some(state),
            _ => None,
        }
    }
}
```

Set `wrapped_native` during PoolManager construction (e.g., in `BacktestRunner::init_pools` or wherever the PoolManager is created).

- [ ] **Step 3: Read the current sandwich detection to find where to add profit computation**

```bash
Get-Content -Path "mev-backtest-core/src/mev/sandwich.rs"
```

- [ ] **Step 4: Add profit computation to SandwichDetector::detect()**

After constructing the frontrun/backrun records and before creating the `MevOpportunity`, add:

```rust
// Compute sandwich profit
let profit_wei = if let Ok(pool_info) = pool_manager.get_pool_info(&pool) {
    // Frontrun buys token, backrun sells — profit in frontrun's input token
    let profit_in_token0 = if frontrun_tx.direction == SwapDirection::Token0ForToken1 {
        // Frontrun paid token0, received token1
        // Backrun paid token1, received token0
        // Profit in token0 = backrun received - frontrun paid
        (backrun_tx.amount_out as i128) - (frontrun_tx.amount_in as i128)
    } else {
        // Frontrun paid token1, received token0
        // Backrun paid token0, received token1
        // Profit in token1 = backrun received - frontrun paid  
        (backrun_tx.amount_out as i128) - (frontrun_tx.amount_in as i128)
    };

    if profit_in_token0 <= 0 {
        U256::ZERO
    } else if pool_manager.is_wrapped_native(&pool_info.token0) {
        U256::from(profit_in_token0 as u128)
    } else if pool_manager.is_wrapped_native(&pool_info.token1) {
        // Convert from token0 denomination to token1 (native) via pool spot price
        // profit_in_native = profit_in_token0 * reserve1 / reserve0
        if let Some(state) = pool_manager.get_v2_state(&pool) {
            if state.reserve0 > 0 {
                let scaled = (profit_in_token0 as u128)
                    .saturating_mul(state.reserve1)
                    .saturating_div(state.reserve0);
                U256::from(scaled)
            } else {
                U256::ZERO
            }
        } else {
            U256::ZERO
        }
    } else {
        U256::ZERO
    }
} else {
    U256::ZERO
};
```

Then replace `expected_profit: U256::ZERO` with `expected_profit: profit_wei`.

- [ ] **Step 5: Add `is_wrapped_native` and `get_v2_state` helper tests**

In `state.rs` tests:
```rust
#[test]
fn test_is_wrapped_native() {
    let mut pm = PoolManager::default();
    // PoolManager needs wrapped_native set — test depends on how it's configured
}
```

- [ ] **Step 6: Run sandwich tests**

```bash
cargo test -p mev-backtest-core sandwich 2>&1
```

Expected: Pass

- [ ] **Step 7: Run all tests**

```bash
cargo test -p mev-backtest-core 2>&1
```

Expected: All pass

- [ ] **Step 8: Commit**

```bash
git add mev-backtest-core/src/mev/sandwich.rs mev-backtest-core/src/pool/state.rs
git commit -m "feat(core): compute sandwich profit from frontrun/backrun amounts"
```

---

### Task 7: Add wei-precise fields to metrics

**Files:**
- Modify: `mev-backtest-core/src/aggregate.rs`
- Test: existing tests (they will need updating if struct layouts change)

- [ ] **Step 1: Read aggregate.rs fully**

```bash
Get-Content -Path "mev-backtest-core/src/aggregate.rs"
```

- [ ] **Step 2: Add wei fields to strategy/dex/summary metrics**

In each metric struct, add wei-precise fields. For example in `StrategyMetrics`:

```rust
pub struct StrategyMetrics {
    // existing f64 fields...
    pub count: usize,
    pub gross_revenue: f64,
    pub net_profit: f64,
    pub total_gas_cost: f64,
    pub avg_profit: f64,
    pub min_profit: f64,
    pub max_profit: f64,
    // new wei fields
    pub gross_revenue_wei: u128,
    pub net_profit_wei: i128,
    pub total_gas_cost_wei: u128,
}
```

Same for `SummaryMetrics` and `DexMetrics`.

- [ ] **Step 3: Update aggregation logic to populate wei fields**

In the aggregation loop, add:
```rust
// Track wei-precise values
sm.gross_revenue_wei = sm.gross_revenue_wei.saturating_add(gross_wei);
sm.net_profit_wei = sm.net_profit_wei.saturating_add(net_wei as i128);
sm.total_gas_cost_wei = sm.total_gas_cost_wei.saturating_add(gas_wei);
```

Where `gross_wei = o.expected_profit.to::<u128>()` and `net_wei = gross_wei - o.gas_cost_wei`.

- [ ] **Step 4: Update aggregate tests to check wei fields**

Find the test that validates aggregation and add assertions:
```rust
#[test]
fn test_aggregation_wei_fields() {
    // existing test setup...
    assert!(result.summary.gross_revenue_wei > 0);
    assert!(result.summary.total_gas_cost_wei > 0);
}
```

- [ ] **Step 5: Run all tests**

```bash
cargo test -p mev-backtest-core 2>&1
```

Expected: All pass

- [ ] **Step 6: Commit**

```bash
git add mev-backtest-core/src/aggregate.rs
git commit -m "feat(core): add wei-precise fields to metrics structs"
```

---

### Task 8: Wire CoinGecko into aggregation, API, and CLI

**Files:**
- Modify: `mev-backtest-core/src/aggregate.rs`
- Modify: `mev-backtest-api/src/mapping.rs`
- Modify: `mev-backtest-cli/src/main.rs`

- [ ] **Step 1: Read current aggregate() signature and update it**

```bash
Get-Content -Path "mev-backtest-core/src/aggregate.rs" | Select-String -Pattern "pub fn aggregate" -Context 0,10
```

Replace the existing `aggregate` function signature:

```rust
// Current:
pub fn aggregate(
    opportunities: &[MevOpportunity],
    chain: ChainName,
) -> AggregationResult {

// New:
pub fn aggregate(
    opportunities: &[MevOpportunity],
    chain: ChainName,
    price_cache: Option<&mut PriceCache>,
) -> AggregationResult {
```

Inside the function body, find where `ETH_USD_RATE` is defined and remove it. Instead, fetch the price dynamically:

```rust
// Remove: const ETH_USD_RATE: f64 = 3200.0;

// Add after the existing aggregation variables:
let usd_price = match price_cache {
    Some(cache) => {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => handle.block_on(cache.usd_price(chain)).unwrap_or(0.0),
            Err(_) => 0.0,
        }
    }
    None => 0.0,
};

// Then replace all references to ETH_USD_RATE with usd_price.
// For example:
// Before: let total_usd = total_eth * ETH_USD_RATE;
// After:  let total_usd = total_eth * usd_price;
```

Also add a default `price_cache: None` to all existing call sites so the code compiles. Search for all calls to `aggregate(` and add the parameter.

- [ ] **Step 2: Update the API mapping to pass price data**

In `mapping.rs`, accept a USD price parameter and use it:
```rust
pub fn map_opportunity(
    opp: &MevOpportunity,
    _pool_registry: &PoolRegistry,
    is_flash_loan: bool,
    block_hash: &str,
    usd_price: f64,  // NEW
) -> UiOpportunity {
    // Use usd_price to compute USD values
    let gross_usd = gross * usd_price;
    // ...
}
```

- [ ] **Step 3: Update CLI main.rs to create and pass PriceCache**

In `mev-backtest-cli/src/main.rs`, in the report/run commands:
```rust
use mev_backtest_core::coingecko::PriceCache;

let mut price_cache = PriceCache::new(config.coingecko_api_key.clone());
```

- [ ] **Step 4: Remove hardcoded ETH_USD_RATE from aggregate.rs**

Find and remove:
```rust
const ETH_USD_RATE: f64 = 3200.0;
```

Replace all usages with the dynamically fetched price.

- [ ] **Step 5: Run full test suite**

```bash
cargo test -p mev-backtest-core -p mev-backtest-api -p mev-backtest-cli 2>&1
```

Expected: All pass

- [ ] **Step 6: Commit**

```bash
git add mev-backtest-core/src/aggregate.rs mev-backtest-api/src/mapping.rs mev-backtest-cli/src/main.rs
git commit -m "feat: wire CoinGecko prices into aggregation, API, and CLI"
```

---

### Task 9: Final cleanup and validation

**Files:** All modified

- [ ] **Step 1: Run full workspace tests**

```bash
cargo test --workspace 2>&1
```

Expected: All pass

- [ ] **Step 2: Run clippy**

```bash
cargo clippy --workspace --all-targets 2>&1
```

Expected: Clean (or minor warnings)

- [ ] **Step 3: Run cargo check on all targets**

```bash
cargo check --workspace --all-targets 2>&1
```

Expected: No errors

- [ ] **Step 4: Verify the example config TOML is updated**

Read `mev-backtest.example.toml` and add the new fields if not present:
```toml
coinGecko API key for USD price conversion (optional)
coingecko_api_key = "CG-..."  # if present, used for USD price lookups; free tier works without

Per-strategy gas limit overrides (optional)
[gas_limits]
two_hop_arb = 150000
multi_hop_arb = 300000
jit = 300000
jit_arb = 350000
sandwich = 200000
```

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "chore: update example config and final cleanup"
```
