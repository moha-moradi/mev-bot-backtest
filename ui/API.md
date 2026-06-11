# MEVSCOPE API Reference

Base URL: `http://localhost:3001`

All endpoints return JSON unless otherwise noted.

---

## Table of Contents

1. [Quick Start](#1-quick-start)
2. [Chains](#2-chains)
3. [Simulate (Run Pipeline)](#3-simulate-run-pipeline)
4. [Real-time Status (SSE)](#4-real-time-status-sse)
5. [Results](#5-results)
6. [History](#6-history)
7. [Export](#7-export)
8. [Data Types](#8-data-types)
9. [Error Handling](#9-error-handling)

---

## 1. Quick Start

### Start the server

```bash
RUST_LOG=info RESULTS_DIR=./results cargo run --package mev-backtest-api
```

### Frontend workflow

1. **Get supported chains** → `GET /api/chains`
2. **Start a simulation** → `POST /api/simulate`
3. **Watch progress** → `GET /api/simulate/{run_id}/status` (SSE stream)
4. **Get results** → `GET /api/simulate/{run_id}/results`
5. **View history** → `GET /api/history`

### Example: Run a simulation

```javascript
// 1. Start simulation
const res = await fetch('http://localhost:3001/api/simulate', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    chain: 'polygon',
    window: { mode: 'single', single_block: 50000000 },
    strategies: ['arb', 'jit']
  })
});
const { run_id } = await res.json();

// 2. Listen to SSE events
const sse = new EventSource(`http://localhost:3001/api/simulate/${run_id}/status`);
sse.addEventListener('stage_start', (e) => console.log('stage start:', JSON.parse(e.data)));
sse.addEventListener('stage_end', (e) => console.log('stage end:', JSON.parse(e.data)));
sse.addEventListener('log', (e) => console.log('log:', JSON.parse(e.data)));
sse.addEventListener('complete', (e) => { console.log('done:', JSON.parse(e.data)); sse.close(); });
sse.addEventListener('error', (e) => { console.log('error:', JSON.parse(e.data)); sse.close(); });

// 3. Poll for results when complete
const results = await fetch(`http://localhost:3001/api/simulate/${run_id}/results`);
const data = await results.json();
```

---

## 2. Chains

### `GET /api/chains`

Returns all supported blockchain networks and their DEX configurations.

**Response:**

```json
[
  {
    "id": "polygon",
    "name": "Polygon",
    "native_token": "MATIC",
    "color": "#8247E5",
    "block_time": 2.0,
    "rpc_default": "https://polygon-bor.publicnode.com",
    "explorer_base": "https://polygonscan.com/tx/",
    "coingecko_id": "matic-network",
    "activity_multiplier": 1.5,
    "avg_tx_per_block": 400.0,
    "gas_price_gwei": 50.0,
    "native_usd": 0.85,
    "dexes": [
      { "id": "uni-v2", "name": "QuickSwap", "fork": "UniV2", "router": "0xa5E0...e38B" },
      { "id": "uni-v3", "name": "Uniswap v3", "fork": "UniV3", "router": "0xE592...5A67" },
      { "id": "sushi", "name": "SushiSwap", "fork": "UniV2", "router": "0x1b02...8Ab2" },
      { "id": "curve", "name": "Curve", "fork": "Curve", "router": "0x7De0...2b3b" }
    ],
    "flash_loan_providers": ["Balancer v2", "Aave v3"]
  }
]
```

**Supported chains:** `ethereum`, `polygon`, `bsc`, `arbitrum`, `avalanche`, `base`, `optimism`

| Field | Type | Description |
|---|---|---|
| `id` | string | Chain identifier |
| `name` | string | Display name |
| `native_token` | string | Native currency symbol |
| `color` | string | Theme color hex |
| `block_time` | number | Seconds between blocks |
| `rpc_default` | string | Default public RPC URL |
| `explorer_base` | string | Base URL for transaction explorer |
| `dexes` | DexConfig[] | Supported DEXes with router addresses |
| `flash_loan_providers` | string[] | Available flash loan sources |
| `native_usd` | number | Approximate USD price of native token |
| `gas_price_gwei` | number | Typical gas price in Gwei |
| `avg_tx_per_block` | number | Average transactions per block |

---

## 3. Simulate (Run Pipeline)

### `POST /api/simulate`

Starts a backtest simulation pipeline in the background. Returns a `run_id` immediately.

**Request body:**

```json
{
  "chain": "polygon",
  "rpc_url": "https://polygon-mainnet.g.alchemy.com/v2/YOUR_KEY",
  "window": { "mode": "single", "single_block": 50000000 },
  "strategies": ["arb", "jit"],
  "flash_loan_provider": "auto",
  "gas_model": "historical_exact",
  "priority_fee_gwei": 0.0,
  "gas_limit": 200000
}
```

| Field | Type | Required | Default | Description |
|---|---|---|---|---|
| `chain` | string | **yes** | — | One of: `ethereum`, `polygon`, `bsc`, `arbitrum`, `avalanche`, `base`, `optimism` |
| `rpc_url` | string | no | chain's `rpc_default` | Custom RPC endpoint |
| `window` | WindowConfig | no | `{ mode: "blocks", last_days: 100 }` | Block range to scan |
| `strategies` | string[] | **yes** | — | At least one of: `arb`, `jit`, `jitarb`, `sandwich` |
| `flash_loan_provider` | string | no | `"auto"` | Flash loan source |
| `gas_model` | string | no | `"historical_exact"` | `historical_exact`, `p90`, or `fixed` |
| `priority_fee_gwei` | number | no | `0.0` | Priority fee (tip) in Gwei |
| `gas_limit` | number | no | `200000` | Gas limit per transaction |

**WindowConfig modes:**

| `mode` value | Fields used | Default | Validation |
|---|---|---|---|
| `"days"` | `last_days` | 7 days | 1–365 |
| `"blocks"` | `last_days` (treated as block count) | 100 blocks | >= 1 |
| `"single"` | `single_block` | — | > 0 |
| `"range"` | `from_block`, `to_block` | — | `to_block > from_block` |

**Strategy mapping:**

| UI name | Strategy scanned |
|---|---|
| `"arb"` | Two-hop arbitrage |
| `"jit"` | Just-in-time liquidity |
| `"jitarb"` | JIT + arbitrage combined |
| `"sandwich"` | Sandwich attack |

**Response (202):**

```json
{
  "run_id": "run_1718000000",
  "status": "pending",
  "created_at": 1718000000
}
```

The pipeline runs asynchronously. Use the SSE status endpoint or poll results with this `run_id`.

---

## 4. Real-time Status (SSE)

### `GET /api/simulate/{run_id}/status`

Server-Sent Events stream. Subscribe to track pipeline progress in real time.

**JavaScript usage:**

```javascript
const sse = new EventSource(`http://localhost:3001/api/simulate/${run_id}/status`);
sse.addEventListener('stage_start', handler);
sse.addEventListener('stage_end', handler);
sse.addEventListener('progress', handler);
sse.addEventListener('log', handler);
sse.addEventListener('complete', handler);
sse.addEventListener('error', handler);
```

**Event types:**

#### `stage_start`

```json
{
  "stage": 0,
  "id": "rpc_fetch",
  "label": "RPC FETCH",
  "sub": "Connecting to polygon"
}
```

#### `stage_end`

```json
{ "stage": 0, "id": "rpc_fetch", "result": "OK" }
{ "stage": 3, "id": "opportunity_scan", "result": "42 opportunities" }
{ "stage": 4, "id": "profitability", "result": "15 profitable" }
```

#### `progress` (during opportunity scan)

```json
{
  "stage": 3,
  "block": 52123456,
  "blocks_processed": 50,
  "total_blocks": 100
}
```

#### `log`

```json
{ "ts": "14:32:01.123", "tag": "RPC", "text": "Connected to polygon (chain 137)" }
```

Tags: `RPC`, `FETCH`, `POOLS`, `GAS`, `SCAN`, `PROFIT`, `SAVE`, `BLOCK`, `ERROR`

#### `complete`

```json
{ "run_id": "run_1718000000", "duration_ms": 12345 }
```

#### `error` (terminal)

```json
{ "stage": 0, "id": "rpc_fetch", "error": "RPC connection failed: ..." }
```

### Pipeline stages

| Stage | ID | Label | What happens |
|---|---|---|---|
| 0 | `rpc_fetch` | RPC FETCH | Connect to RPC, resolve block range |
| 1 | `tx_filter` | TX FILTER | Fetch blocks (cache-first) |
| 2 | `revm_replay` | REVM REPLAY | Initialize pools, build replayer |
| 3 | `opportunity_scan` | OPPORTUNITY SCAN | Scan each block for MEV (slowest) |
| 4 | `profitability` | PROFITABILITY CHECK | Filter opportunities with profit > 0 |
| 5 | `aggregation` | AGGREGATION | Compute metrics, save to disk |

---

## 5. Results

### `GET /api/simulate/{run_id}/results`

Returns the simulation result or current progress if still running.

**Response (completed):**

```json
{
  "run_id": "run_1718000000",
  "chain": "polygon",
  "start_block": 50000000,
  "end_block": 50000100,
  "strategies": ["arb", "jit"],
  "opportunities": [ /* UiOpportunity[] */ ],
  "summary": { /* SummaryMetrics */ },
  "by_strategy": { /* map of strategy → StrategyMetrics */ },
  "by_dex": [ /* DexMetrics[] */ ],
  "duration_ms": 12345,
  "created_at": 1718000000
}
```

**Response (still running):**

```json
{
  "run_id": "run_1718000000",
  "status": "running",
  "progress": 45.0,
  "blocks_processed": 45,
  "blocks_total": 100,
  "stages": [
    { "id": "rpc_fetch", "label": "RPC FETCH", "status": "Completed" },
    { "id": "tx_filter", "label": "TX FILTER", "status": "Completed" },
    { "id": "revm_replay", "label": "REVM REPLAY", "status": "Completed" },
    { "id": "opportunity_scan", "label": "OPPORTUNITY SCAN", "status": "Running" },
    { "id": "profitability", "label": "PROFITABILITY CHECK", "status": "Pending" },
    { "id": "aggregation", "label": "AGGREGATION", "status": "Pending" }
  ],
  "logs": [ /* LogEntry[] */ ],
  "message": "results not available yet"
}
```

**Response (error):**

```json
{
  "run_id": "run_1718000000",
  "status": "error",
  "error": "RPC connection failed: ...",
  "logs": [ /* LogEntry[] */ ]
}
```

**Possible status values:** `pending`, `running`, `done`, `error`, `cancelled`

---

## 6. History

### `GET /api/history`

Lists all completed simulation runs from disk (sorted newest first).

**Response:**

```json
[
  {
    "id": "run_1718000000",
    "started_at": 1718000000,
    "duration_ms": 12345,
    "chain_id": "polygon",
    "window_summary": "single",
    "enabled_strategies": ["arb", "jit"],
    "opportunities": 42,
    "net_profit": 0.0
  }
]
```

### `GET /api/history/{run_id}`

Returns the full result JSON for a completed run.

### `DELETE /api/history/{run_id}`

Deletes a saved result file from disk.

**Response:**

```json
{ "deleted": "run_1718000000" }
```

---

## 7. Export

### `GET /api/export/{run_id}/json`

Downloads the result as a JSON file (`Content-Disposition: attachment`).

### `GET /api/export/{run_id}/csv`

Downloads opportunities as CSV with these columns:

```
tx_hash, block_number, timestamp, strategy, gross_revenue, gas_cost, flash_loan_fee, builder_tip, net_profit, result, token_pair, dex_path
```

The `dex_path` field uses `;` as separator when multiple DEXes are involved.

---

## 8. Data Types

### UiOpportunity

| Field | Type | Description |
|---|---|---|
| `id` | string | Unique opportunity ID |
| `tx_hash` | string | Transaction hash |
| `block_number` | number | Block where opportunity was found |
| `timestamp` | number | Block timestamp (Unix seconds) |
| `strategy` | string | `"arb"`, `"jit"`, `"jitarb"`, or `"sandwich"` |
| `gross_revenue` | number | Gross revenue in native token |
| `gas_cost` | number | Gas cost in native token |
| `flash_loan_fee` | number | Flash loan fee |
| `builder_tip` | number | Builder/MEV tip |
| `net_profit` | number | `gross_revenue - gas_cost - flash_loan_fee - builder_tip` |
| `result` | string | `"profitable"`, `"below_threshold"`, or `"reverted"` |
| `explorer_url` | string | Link to block explorer |
| `token_pair` | string? | Token pair involved |
| `dex_path` | string[]? | Path of DEXes used |
| `pool_a` | string? | First pool address (truncated) |
| `pool_b` | string? | Second pool address (truncated) |
| `input_amount` | string? | Input amount with decimals |
| `flash_loan_provider` | string? | Flash loan source |
| `flash_loan_size` | number? | Flash loan amount |
| `victim_tx_hash` | string? | Victim's transaction hash (sandwich) |
| `front_run_size` | number? | Front-run amount (sandwich) |
| `victim_slippage` | number? | Victim slippage tolerance (sandwich) |
| `gross_capture` | number? | Gross capture ratio |
| `simulation_trace` | SimulationTrace | Step-by-step simulation trace |

### SimulationTrace

```json
{
  "title": "Arbitrage trace",
  "steps": [
    { "label": "Buy on QuickSwap", "value": null, "sub": null },
    { "label": "Sell on SushiSwap", "value": "100.50 MATIC", "sub": "price: 1.005" }
  ],
  "result": { "gross": 100.5, "cost": 0.05, "net": 100.45 }
}
```

| Title pattern | Strategy |
|---|---|
| `"Arbitrage trace"` | arb |
| `"JIT Liquidity trace"` | jit |
| `"Sandwich trace"` | sandwich |
| `"JIT+Arb trace"` | jitarb |

### SummaryMetrics

| Field | Type | Description |
|---|---|---|
| `total` | number | Total opportunities found |
| `profitable` | number | Count with `expected_profit > 0` |
| `gross_revenue` | number | Total gross revenue in native token |
| `net_profit` | number | Total net profit in native token |
| `net_profit_usd` | number | Net profit in USD (estimated) |
| `total_cost` | number | Total gas cost in native token |
| `best_strategy` | string? | Strategy with highest net profit |
| `best_single_opp` | number | Highest single gross revenue |

### StrategyMetrics

| Field | Type | Description |
|---|---|---|
| `strategy` | string | Strategy name |
| `count` | number | Opportunities found |
| `profitable` | number | Profitable count |
| `gross_revenue` | number | Total gross revenue |
| `gas_fees` | number | Total gas fees |
| `net_profit` | number | Net profit |
| `net_profit_usd` | number | Net profit in USD |
| `roi` | number | ROI % = `(net_profit / gas_fees) * 100` |
| `avg_per_opp` | number | Average profit per opportunity |
| `best_opp` | number | Best single opportunity profit |

### DexMetrics

| Field | Type | Description |
|---|---|---|
| `dex` | string | DEX name |
| `fork` | string | Fork type (`UniV2`, `UniV3`, `Curve`, `Solidly`) |
| `tx_count` | number | Transactions involving this DEX |
| `opportunities` | number | Opportunities using this DEX |
| `profitable` | number | Profitable opportunities |
| `revenue` | number | Total revenue from this DEX |
| `avg_profit` | number | Average profit per opportunity |

### StageStatus

One of: `Pending`, `Running`, `Completed`, `Skipped`, `Failed`

### LogEntry

```json
{ "ts": "14:32:01.123", "tag": "RPC", "text": "Connected to polygon (chain 137)" }
```

---

## 9. Error Handling

All errors return JSON with appropriate HTTP status code.

### 400 Bad Request

```json
{ "error": "unknown chain 'solana'. Supported: ethereum, polygon, bsc, arbitrum, avalanche, base, optimism" }
{ "error": "chain 'xxx' not configured" }
{ "error": "at least one valid strategy required (arb, jit, jitarb, sandwich)" }
{ "error": "days must be between 1 and 365" }
{ "error": "single_block must be > 0" }
{ "error": "to_block must be greater than from_block" }
{ "error": "unknown window mode 'xxx'" }
```

### 404 Not Found

```json
{ "error": "run not found" }
```

### 500 Internal Server Error

```json
{ "error": "failed to read results dir: ..." }
{ "error": "parse error: ..." }
{ "error": "delete failed: ..." }
```

---

## CORS

Configured to allow `http://localhost:8080` origin with all methods and headers.
