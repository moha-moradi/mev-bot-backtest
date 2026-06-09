# Real Pipeline Integration Spec

> **Date:** 2026-06-09
> **Status:** Draft

## Goal

Replace placeholder pipeline stages in `POST /api/simulate` with real backtest execution: RPC connection, block range resolution, cache-first fetching, pool initialization, REVM replay, MEV detection, aggregation, and results persistence.

## Architecture

A `PipelineOrchestrator` struct in `mev-backtest-api/src/pipeline.rs` owns the entire backtest execution lifecycle. It is spawned as a `tokio::spawn` task from the simulate handler, receives a `broadcast::Sender<SseEvent>` for SSE progress and an `Arc<RwLock<RunState>>` for state tracking.

The orchestrator uses existing `BacktestRunner::run_block()` for the per-block MEV detection loop, emitting SSE events between blocks. It does NOT modify any code in `mev-backtest-core` — it only depends on public APIs.

## Pipeline Stages

### Stage 0: RPC Connection & Range Resolution
- Parse `chain_id` from request → look up `ChainConfig` from API's chain metadata
- Build `RpcClient` from provided RPC URL
- Check RPC connection with `rpc.check_connection(chain_id)`
- Resolve block range using `RangeResolver`:
  - If `from_block` + `to_block` provided → use directly
  - If `blocks` (count from tip) → `RangeResolver::resolve(Blocks(n))`
  - If `days` → `RangeResolver::resolve(Days(n))`
- Emit `stage_0_complete` SSE event

### Stage 1: Block Fetching (Cache-First, RPC Fallback)
- Open or create `CacheStore` at configurable cache directory
- Clone `CacheStore` — one for fetcher, one for replayer
- Create `Fetcher` with RPC + cache clone
- Call `fetcher.fetch_range(&resolved, Some(&log_fn))` — the log function emits SSE progress per batch
- Log summary: total blocks, fetched, cached, missing
- Emit `stage_1_complete` SSE event

### Stage 2: Pool Initialization & Replayer Construction
- Create empty `PoolManager`
- Call `BacktestRunner::init_pools(&mut pool_manager, registry_path, &rpc, prev_block, Some(&cache))` to load registry pools + discovered pools + fetch reserves
- Build `BlockReplayer` with `tokio::runtime::Handle::current()`, cache clone, rpc, chain_id
- Create `BacktestRunner::new(replayer, pool_manager, gas_config)`
- Emit `stage_2_complete` SSE event

### Stage 3: Opportunity Scanning (Per-Block)
- Iterate `block_num` from `resolved.start_block` to `resolved.end_block`
- For each block:
  - Call `runner.run_block(block_num)` (synchronous, uses `block_in_place` internally)
  - Extend accumulated opportunities
  - Update `RunState.progress` (blocks_processed + 1, blocks_total)
  - Emit `block_complete` SSE with block number, count so far
  - Check for cancellation (`RunState.status == Cancelling`)
- Emit `stage_3_complete` SSE event

### Stage 4: Profitability Filtering
- Filter opportunities where `net_profit` > threshold (default 0)
- Update state with filtered count
- Emit `stage_4_complete` SSE event

### Stage 5: Aggregation & Persistence
- Call `aggregate::aggregate(&opportunities)` to produce `AggregationResult`
- Map `MevOpportunity` → `UiOpportunity` via `mapping::to_ui_opportunities()`
- Build `RunResult` with summary + ui_opportunities + traces + aggregation
- Save results to `./results/{run_id}.json`
- Set `RunState.status = Done`, `RunState.result = Some(RunResult)`
- Emit `complete` SSE event

## SSE Events

```rust
pub struct SseEvent {
    pub stage: String,       // "0".."5" or "complete"
    pub status: String,      // "running" | "complete" | "error"
    pub message: String,
    pub progress: Option<StageProgress>,
    pub opportunities_found: Option<usize>,
    pub error: Option<String>,
}

pub struct StageProgress {
    pub blocks_processed: u64,
    pub blocks_total: u64,
    pub elapsed_secs: f64,
}
```

## Error Handling

If any stage fails (RPC error, replayer error, etc.):
- Set `RunState.status = Error`, `RunState.error = Some(error_string)`
- Emit `error` SSE event
- The `GET /results/{run_id}` endpoint returns the error message
- The `GET /status/{run_id}` SSE stream emits the error event and then closes

## SimulateRequest Format

The existing `POST /api/simulate` request body already specifies all needed fields:

```json
{
  "chain": "polygon",
  "rpc_url": "https://polygon-rpc.com",
  "window": {
    "mode": "range",
    "from_block": 50000000,
    "to_block": 50000100
  },
  "strategies": ["arb", "jit", "jitarb", "sandwich"],
  "flash_loan_provider": "balancer",
  "gas_model": "historical_exact",
  "priority_fee_gwei": 30.0,
  "gas_limit": 5000000
}
```

The `window.mode` field accepts: `"days"`, `"blocks"`, `"single"`, `"range"`.

## Changes to Existing Files

### `mev-backtest-api/src/routes/simulate.rs`
- Replace placeholder background task with call to `PipelineOrchestrator::run()`
- Build the config from request parameters (range mode, strategies, gas config, flash loan provider)
- Create run_id via UUID
- Create RunState, SSE broadcast channel, register in AppState
- Spawn orchestrator task

### `mev-backtest-api/src/state.rs`
- Add `blocks_processed: u64` and `blocks_total: u64` to RunState for per-block progress tracking

### `mev-backtest-api/src/routes/mod.rs`
- Export `pipeline` module (add `pub mod pipeline`)

### `mev-backtest-api/src/main.rs`
- Read `RESULTS_DIR` env var or default to `./results`
- Create results directory on startup

## No Changes Required

- `mev-backtest-core/` — no changes to core crate
- `mev-backtest-api/src/mapping.rs` — already correct
- `mev-backtest-api/src/routes/status.rs` — SSE streaming already implemented
- `mev-backtest-api/src/routes/results.rs` — result lookup already implemented
- `mev-backtest-api/src/routes/history.rs` — history already uses disk-based results
- `mev-backtest-api/src/routes/export.rs` — file download already implemented
- `mev-backtest-api/src/main.rs` — no route changes needed

## Test Strategy

- Manual integration test: start API server, POST to `/api/simulate` with polygon RPC + from_block + to_block, observe SSE events, verify results JSON on disk
- Unit test: test orchestrator helper functions (config building, event emission)

## File Structure (Final)

```
mev-backtest-api/src/
├── main.rs
├── state.rs
├── mapping.rs
├── pipeline.rs          ← NEW
└── routes/
    ├── mod.rs
    ├── chains.rs
    ├── simulate.rs      ← MODIFIED
    ├── status.rs
    ├── results.rs
    ├── history.rs
    └── export.rs
```
