# Phase 4: TwoHopArb Detection Engine for Polygon

**Builds on**: Phase 3 (EVM State Replay Engine)
**Goal**: Transform replay engine into a production backtester that detects two-hop arbitrage on Polygon V2 DEXes (QuickSwap, SushiSwap).
**Target**: Polygon mainnet (chain_id=137). V2 constant-product AMMs only.

## Scope

- **Strategy**: TwoHopArb only
- **Pool type**: Uniswap V2–style constant product AMM
- **Pool count**: top 50 pools (Polygon)
- **Flash loan**: simplified (math-based, no callback simulation)

## Architecture

```
Flow per block:
  PoolManager.init(block_num - 1) → fetch initial reserves via eth_call

  for tx_index in 0..block.tx_count:
    (state, executed_txs) = replayer.replay_to(block_num, tx_index)
    PoolManager.update_from_logs(executed_txs.last().logs)   // Swap/Sync events

    for each (pool_a, pool_b) where they share a token:
      if let Some(opp) = TwoHopArbDetector.detect(pool_a, pool_b):
        opportunities.push(opp)
```

## New Files & Modules

### Pool Module (`mev-backtest-core/src/pool/`)

| File | Content |
|------|---------|
| `pool/mod.rs` | Module exports |
| `pool/math.rs` | `constant_product_output_amount()`, `optimal_two_hop_arb()` (ternary search) |
| `pool/state.rs` | `PoolInfo`, `UniswapV2PoolState`, `PoolState`, `PoolManager` |
| `pool/registry.rs` | `PoolRegistry::load(path) → Vec<PoolInfo>` |

### MEV Module (`mev-backtest-core/src/mev/`)

| File | Content |
|------|---------|
| `mev/mod.rs` | Module exports |
| `mev/opportunity.rs` | `MevOpportunity` struct |
| `mev/two_hop.rs` | `TwoHopArbDetector::detect(pool_manager) → Vec<MevOpportunity>` |

### Backtest Runner

| File | Content |
|------|---------|
| `run.rs` | `BacktestRunner::run_block()`, `run_range()` |

## Modified Files

| File | Change |
|------|--------|
| `mev-backtest-core/src/lib.rs` | Add `pub mod pool; pub mod mev; pub mod run;` |
| `mev-backtest-core/src/rpc.rs` | Add `call(to, data, block)` for eth_call |
| `mev-backtest-cli/src/main.rs` | Wire `Command::Run` with actual backtest logic |
| `mev-backtest-core/src/validation.rs` | Update Run validation to check pool registry |

## Pool JSON (`pools/polygon.json`)

Top-50 Uniswap V2–style pools (QuickSwap, SushiSwap) on Polygon:
```json
[
  {
    "address": "0x...",
    "type": "uniswap_v2",
    "token0": "0x...",
    "token1": "0x...",
    "fee": 30,
    "name": "WMATIC/USDC"
  }
]
```

## Key Technical Details

- **Swap/Sync decoding**: Manual ABI parsing (no sol! macros). Swap = 4 × uint256 packed; Sync = 2 × uint256
- **Reserve init**: `eth_call getReserves()` at block N-1 via `RpcClient::call()`
- **Optimal input**: Ternary search over [0, min(rA, rB)] in 60 iterations
- **Flash loan**: Assume zero-fee flash swap. Arbitrage = buy on A → sell on B
- **Gas estimate**: 200,000 gas/tx. Cost = gas * (base_fee + priority_fee)

## Implementation Order

1. `pool/math.rs` — pure AMM functions (testable immediately)
2. `pool/state.rs` — type definitions
3. `pool/registry.rs` — pool JSON loader
4. `rpc.rs` — add `call()` for eth_call
5. `pool/mod.rs` — PoolManager (init + update)
6. `mev/opportunity.rs` — result type
7. `mev/two_hop.rs` — detection algorithm
8. `run.rs` — backtest orchestrator
9. `lib.rs` + CLI `main.rs` — integration
10. `pools/polygon.json` — pool addresses
11. Tests — math unit tests, integration on known arb block

## Edge Cases

- Pool with zero reserves: skip
- Stale reserves: re-init from RPC periodically
- Both directions per pair (A→B→A, B→A→B)
- Net profit after gas must be > 0 to record
- Token decimals for USD price estimates
