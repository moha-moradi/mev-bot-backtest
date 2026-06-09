# MEVSCOPE API — Backend Implementation Plan

## Overview

Connect the **MEVSCOPE UI** (TanStack Start React app at `D:\gitlab.dte.repo\mev-scout`) to the **mev-bot-backtest Rust engine** (`D:\gitlab.dte.repo\mev-bot-backtest`).

The UI is fully functional with mocked data. The Rust engine is a CLI-only binary. This plan describes the HTTP API needed to serve real data.

---

## 1. New Crate: `mev-backtest-api`

Create a new workspace crate `mev-backtest-api/`.

**Dependencies:** `axum`, `tokio`, `serde`, `serde_json`, `mev-backtest-core`, `tower-http` (CORS), `uuid`, `tracing`, `chrono`

### Endpoints

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/api/health` | Health check |
| `GET` | `/api/chains` | Return supported chains + DEXes + configs |
| `POST` | `/api/simulate` | Start a backtest run |
| `GET` | `/api/simulate/{run_id}/status` | Pipeline progress + logs (SSE) |
| `GET` | `/api/simulate/{run_id}/results` | Full results (opportunities + analytics) |
| `GET` | `/api/history` | List past runs |
| `GET` | `/api/history/{run_id}` | Get a past run's full results |
| `DELETE` | `/api/history/{run_id}` | Delete a past run |
| `GET` | `/api/export/{run_id}/json` | Download results as JSON file |
| `GET` | `/api/export/{run_id}/csv` | Download opportunities as CSV file |

---

## 2. Chain Config Endpoint

### `GET /api/chains`

Maps Rust chain configs to the UI's `ChainConfig` type.

**UI expects** (`src/lib/chains.ts`):

```ts
interface ChainConfig {
  id: string;              // "ethereum", "polygon", etc.
  name: string;            // "Ethereum"
  nativeToken: string;     // "ETH"
  color: string;           // "#627EEA"
  blockTime: number;       // seconds
  rpcDefault: string;      // public RPC URL
  explorerBase: string;    // "https://etherscan.io/tx/"
  dexes: DexConfig[];      // { id, name, fork, router }
  flashLoanProviders: string[];
  coingeckoId: string;
  activityMultiplier: number;
  avgTxPerBlock: number;
  gasPriceGwei: number;
  nativeUSD: number;
}

interface DexConfig {
  id: string;            // "uni-v2", "sushi", etc.
  name: string;          // "Uniswap v2"
  fork: "UniV2" | "UniV3" | "Curve" | "Balancer" | "Solidly" | "Algebra";
  router: string;        // truncated address, e.g. "0x7a25...488D"
}
```

**Source:** `mev-backtest-core/src/config.rs` has built-in per-chain configs with RPC URLs, balancer vault, Aave pool, factory addresses. The `chain.rs` file in the UI also has per-chain metadata.

**Action:** Serve a static JSON mapping. Fields like `color`, `coingeckoId`, `activityMultiplier`, `avgTxPerBlock`, `gasPriceGwei`, `nativeUSD`, `explorerBase` are UI metadata not in the Rust config — either add to config or hardcode in the API.

---

## 3. Simulate Endpoint

### `POST /api/simulate`

**Request body** (maps to UI's Zustand store config):

```json
{
  "chain": "ethereum",
  "rpc_url": "https://eth.llamarpc.com",
  "window": {
    "mode": "days",
    "last_days": 30,
    "from_block": null,
    "to_block": null,
    "single_block": null
  },
  "strategies": ["arb", "jit", "jitarb"],
  "dexes": ["uni-v2", "uni-v3", "sushi", "curve"],
  "flash_loan_provider": "balancer",
  "strategy_params": {
    "arb": {
      "min_spread": 0.3,
      "max_hops": 2,
      "token_whitelist": ["WETH", "USDC", "USDT", "DAI"]
    },
    "jit": {
      "min_tvl": 500000,
      "tick_width": 20,
      "target_pools": ["uni-v3"]
    },
    "jitarb": {
      "flash_provider": "balancer",
      "max_loan_size": 50,
      "min_spread_after_fees": 0.5
    },
    "sandwich": {
      "front_run_size": 1.0,
      "max_victim_slippage": 0.5,
      "fee_tier": "0.3",
      "gas_multiplier": 1.5,
      "min_net_profit": 0.01
    }
  },
  "gas_model": "historical_exact",
  "priority_fee_gwei": 0.0,
  "gas_limit": 200000
}
```

**Strategy name mapping:**

| UI `StrategyId` | Rust `Strategy` | Notes |
|----------------|-----------------|-------|
| `arb` | `TwoHopArb` + `MultiHopArb` | UI treats both as one |
| `jit` | `Jit` | |
| `jitarb` | `JitArb` | |
| `sandwich` | `Sandwich` | |
| `longtail` | ❌ Not implemented | Skip if requested |
| `aggregator` | ❌ Not implemented | Skip if requested |

**Response** (202 Accepted):

```json
{
  "run_id": "run_1780900000",
  "status": "pending",
  "created_at": 1780900000
}
```

**Implementation:**

1. Parse request → internal `Config` struct
2. Resolve block range via existing `resolver.rs`
3. Spawn background `tokio::task` with `BacktestRunner::run_range()`
4. Store in `HashMap<String, RunState>` (behind `Arc<RwLock<>>`)

---

## 4. Pipeline SSE Streaming

### `GET /api/simulate/{run_id}/status`

Use **Server-Sent Events (SSE)** to stream progress in real time.

**Event types:**

```
event: stage_start
data: {"stage": 0, "id": "rpc_fetch", "label": "RPC FETCH", "sub": "Fetching block range on Ethereum"}

event: log
data: {"ts": "12:34:56.789", "tag": "RPC", "text": "Fetched block #19842000 (12 txs)"}

event: progress
data: {"stage": 2, "progress": 45, "elapsed": 3200, "eta": 4200}

event: stage_end
data: {"stage": 0, "id": "rpc_fetch", "result": "OK"}

event: complete
data: {"run_id": "run_1780900000", "opportunities": 42, "duration_ms": 8500}

event: error
data: {"run_id": "run_1780900000", "error": "RPC connection failed"}
```

**Log tags** (color-coded by UI's `LiveLog.tsx`):

| Tag | Color | When emitted |
|-----|-------|-------------|
| `RPC` | default | Block fetch progress |
| `FILTER` | default | TX filtering progress |
| `REPLAY` | default | REVM replay progress per tx |
| `SCAN` | blue | Per-strategy candidate found |
| `FLASH` | purple | Flash loan simulation |
| `PROFIT` | default | Profitability check |
| `AGG` | default | Aggregation |
| `DONE` | green | Stage complete |
| `SKIP` | muted | Stage skipped |

**Pipeline stages** (5 stages, matches UI):

| # | ID | Label | Sub template | Implementation |
|---|----|-------|-------------|----------------|
| 0 | `rpc_fetch` | RPC FETCH | `"Fetching {mode} block window from {chain} RPC"` | `fetch::fetch_range_parallel()` |
| 1 | `tx_filter` | TX FILTER | `"Filtering DEX swaps across {N} DEXes"` | Filter txs by tracked pools |
| 2 | `revm_replay` | REVM REPLAY | `"Replaying transactions (REVM · 8 threads)"` | `BlockReplayer::replay_each_filtered()` |
| 3 | `opportunity_scan` | OPPORTUNITY SCAN | `"Scanning: {strategies}"` | Run all enabled detectors |
| 4 | `profitability` | PROFITABILITY CHECK | `"Checking net profit ≥ threshold per strategy"` | Filter + compute net |
| 5 | `aggregation` | AGGREGATION | `"Computing P&L across {N} strategies"` | Build summary analytics |

**Per-stage log generation** during `opportunity_scan`:

- For `two_hop_arb`/`multi_hop_arb`: `"SCAN: ARB candidate at block #19,842,042"`
- For `jit`: `"SCAN: JIT opportunity at block #19,842,310"`
- For `sandwich`: `"SCAN: Sandwich candidate: victim 0xabcd… slippage 0.8%"`
- For `jitarb`: `"SCAN: JIT+ARB candidate at block #19,842,318"`

**Implementation** — `RunState` struct:

```rust
struct RunState {
    run_id: String,
    config: Config,
    status: RunStatus,           // Pending | Running | Done | Error
    stages: Vec<StageState>,
    logs: Vec<LogEntry>,
    progress: f64,
    elapsed_ms: u64,
    started_at: u64,
    sse_tx: broadcast::Sender<SseEvent>,
    result: Option<RunResult>,
}
```

**Note:** `RunState` must survive server restarts for active runs — use `Arc<RwLock<HashMap<String, RunState>>>` in app state.

---

## 5. Results Endpoint

### `GET /api/simulate/{run_id}/results`

Returns the full simulation output. The UI's `Opportunity` type expects these fields:

```json
{
  "run_id": "run_1780900000",
  "chain": "ethereum",
  "start_block": 19842000,
  "end_block": 19843000,
  "strategies": ["arb", "jit", "jitarb"],

  "opportunities": [
    {
      "id": "arb-42-19842042",
      "tx_hash": "0xabcd...1234",
      "block_number": 19842042,
      "timestamp": 1780800000,
      "strategy": "arb",
      "gross_revenue": 0.015,
      "gas_cost": 0.003,
      "flash_loan_fee": 0.000014,
      "builder_tip": 0.0015,
      "net_profit": 0.0105,
      "result": "profitable",
      "explorer_url": "https://etherscan.io/tx/0xabcd...1234",

      "token_pair": "WETH/USDC",
      "dex_path": ["Uniswap v2", "SushiSwap"],
      "pool_a": "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",
      "pool_b": "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F",
      "input_amount": "1000000000000000000",

      "flash_loan_provider": "Balancer v2",
      "flash_loan_size": 60.0,

      "sandwich": null,
      "simulation_trace": {
        "title": "Arbitrage trace",
        "steps": [
          { "label": "Block", "value": "#19,842,042" },
          { "label": "Token pair", "value": "WETH/USDC" },
          { "label": "Path", "value": "Uniswap v2 → SushiSwap" },
          { "label": "Flash loan", "value": "60.0 via Balancer v2" },
          { "label": "Gross revenue", "value": "0.01500" },
          { "label": "Gas", "value": "−0.00300" }
        ],
        "result": { "gross": 0.015, "cost": 0.003, "net": 0.0105 }
      }
    }
  ],

  "summary": {
    "total": 42,
    "profitable": 25,
    "gross_revenue": 2.15,
    "net_profit": 0.87,
    "net_profit_usd": 2784.0,
    "total_cost": 0.64,
    "best_strategy": "arb",
    "best_single_opp": 0.12
  },

  "by_strategy": {
    "arb": {
      "strategy": "arb",
      "count": 18,
      "profitable": 12,
      "gross_revenue": 1.2,
      "gas_fees": 0.18,
      "net_profit": 0.52,
      "net_profit_usd": 1664.0,
      "roi": 43.3,
      "avg_per_opp": 0.029,
      "best_opp": 0.12
    }
  },

  "by_dex": [
    {
      "dex": "Uniswap v2",
      "fork": "UniV2",
      "tx_count": 24,
      "opportunities": 12,
      "profitable": 8,
      "revenue": 0.85,
      "avg_profit": 0.071
    }
  ]
}
```

### The `Opportunity` type in full (from UI `mockData.ts`):

```ts
interface Opportunity {
  id: string;
  txHash: string;
  blockNumber: number;
  timestamp: number;
  strategy: StrategyId;           // "arb" | "jit" | "jitarb" | "sandwich" | "longtail" | "aggregator"
  grossRevenue: number;
  gasCost: number;
  flashLoanFee: number;
  builderTip: number;
  netProfit: number;
  result: "profitable" | "below_threshold" | "reverted";
  explorerUrl: string;

  // Arbitrage / JIT / JIT+Arb
  dexPath?: string[];
  tokenPair?: string;
  flashLoanProvider?: string;
  flashLoanSize?: number;

  // Sandwich
  victimTxHash?: string;
  frontRunSize?: number;
  victimSlippage?: number;
  grossCapture?: number;

  // Simulation trace (required for opportunity detail modal)
  simulationTrace: {
    title: string;
    steps: { label: string; value?: string; sub?: string }[];
    result: { gross: number; cost: number; net: number };
  };
}
```

### Net profit computation

The Rust `MevOpportunity` only has `expected_profit` (gross) and `gas_cost_wei`. The UI needs more:

```rust
fn compute_opportunity_ui(
    opp: &MevOpportunity,
    chain: &ChainConfig,
    pool_registry: &PoolRegistry,
    is_flash_loan: bool,
    builder_tip_pct: f64, // e.g. 0.10 = 10%
) -> UiOpportunity {
    let expected_profit_eth = wei_to_eth(opp.expected_profit);
    let gas_cost_eth = wei_to_eth(U256::from(opp.gas_cost_wei));
    let flash_loan_fee = if is_flash_loan { expected_profit_eth * 0.0009 } else { 0.0 };
    let builder_tip = expected_profit_eth * builder_tip_pct;
    let net = expected_profit_eth - gas_cost_eth - flash_loan_fee - builder_tip;

    // strategy -> UI strategy id mapping
    let strategy_id = match opp.strategy {
        Strategy::TwoHopArb | Strategy::MultiHopArb => "arb",
        Strategy::Jit => "jit",
        Strategy::JitArb => "jitarb",
        Strategy::Sandwich => "sandwich",
    };

    // Build simulation trace
    let trace = build_trace(opp, expected_profit_eth, gas_cost_eth, net);

    // Derive token pair, dex path from pool addresses and registry
    let (token_pair, dex_path) = resolve_pool_names(opp, pool_registry);

    UiOpportunity { /* map all fields */ }
}
```

### Simulation trace builder

Each strategy needs a different trace structure (from `mockData.ts` `buildTrace()`):

- **arb**: Block → Token pair → Path → Flash loan (if any) → Gross → Gas
- **jit**: Block → Target pool → Incoming swap detected → Mint LP → Burn LP → Fees earned → Gas
- **jitarb**: Block → Flash loan → JIT mint → Victim swap → Burn LP/arb exit → Repay → Gross → Gas+FL fee
- **sandwich**: Block → Victim tx → Front-run → Victim executes → Back-run → Gross capture → Gas → DEX fees

---

## 6. Aggregation Module

Add to `mev-backtest-core/src/aggregate.rs`:

```rust
pub struct SummaryMetrics {
    pub total: usize,
    pub profitable: usize,
    pub gross_revenue: f64,
    pub net_profit: f64,
    pub net_profit_usd: f64,
    pub total_cost: f64,
    pub best_strategy: Option<String>,
    pub best_single_opp: f64,
}

pub struct StrategyMetrics {
    pub strategy: String,
    pub count: usize,
    pub profitable: usize,
    pub gross_revenue: f64,
    pub gas_fees: f64,
    pub net_profit: f64,
    pub net_profit_usd: f64,
    pub roi: f64,
    pub avg_per_opp: f64,
    pub best_opp: f64,
}

pub struct DexMetrics {
    pub dex: String,
    pub fork: String,
    pub tx_count: usize,
    pub opportunities: usize,
    pub profitable: usize,
    pub revenue: f64,
    pub avg_profit: f64,
}

pub fn aggregate(opportunities: &[MevOpportunity], chain: &ChainConfig, dexes: &[DexMeta]) -> AggregationResult;
```

Both the API and CLI should use this module.

---

## 7. Data Model Translation

### Rust `MevOpportunity` → UI `Opportunity`

| Rust | UI | Notes |
|------|----|-------|
| `block_number: u64` | `blockNumber: number` | Direct cast |
| `tx_index: usize` | — | Used internally for trace |
| `strategy: Strategy` | `strategy: StrategyId` | Map: `TwoHopArb/MultiHopArb` → `"arb"`, `Jit` → `"jit"`, `JitArb` → `"jitarb"`, `Sandwich` → `"sandwich"` |
| `pool_a: Address` | `poolA: string` | hex |
| `pool_b: Address` | `poolB: string` | hex |
| `token_in: Address` | `tokenPair: string` | Resolve symbol from registry |
| `token_out: Address` | — | Used for `tokenPair` |
| `input_amount: U256` | `inputAmount: string` | Decimal string |
| `expected_profit: U256` | `grossRevenue: number` | wei → ETH |
| `gas_cost_wei: u128` | `gasCost: number` | wei → ETH |
| `timestamp: u64` | `timestamp: number` | Unix → JS timestamp |
| — | `flashLoanFee: number` | Compute: `gross × 0.0009` if FL strategy |
| — | `builderTip: number` | Compute: `gross × 0.10` |
| — | `netProfit: number` | `gross - gasCost - flashLoanFee - builderTip` |
| — | `result: string` | `netProfit > minProfit ? "profitable" : "below_threshold"` |
| — | `explorerUrl: string` | `chain.explorerBase + txHash` |
| — | `simulationTrace: object` | Build per strategy type |
| `tick_lower/upper` | — | JIT position range |
| `liquidity_amount` | — | JIT liquidity |
| `victim_tx_index` | — | Sandwich victim index |
| `backrun_tx_index` | — | Sandwich backrun index |
| `path` | `dexPath: string[]` | Map Address[] → pool name[] |

---

## 8. History Endpoint

### `GET /api/history`

List past runs. The UI reads from its `history` array in Zustand, which stores full `SimulationRun` objects:

```json
[
  {
    "id": "run_1780900000",
    "started_at": 1780900000,
    "duration_ms": 8500,
    "chain_id": "ethereum",
    "window_summary": "Last 30 days",
    "enabled_strategies": ["arb", "jit", "jitarb"],
    "opportunities": 42,
    "net_profit": 0.87,
    "summary": { ... },
    "by_strategy": { ... },
    "by_dex": [ ... ],
    "opps": [ ... ],
    "config": { ... },
    "auto_params": { ... }
  }
]
```

**Implementation:** Persist completed `RunResult` as JSON in `./results/{run_id}.json`. The Rust CLI already does this — reuse the same format but extended to include summary + by_strategy + by_dex data.

### `DELETE /api/history/{run_id}`

Delete the result file from disk.

---

## 9. Export Endpoints

### `GET /api/export/{run_id}/json`

Download the full results file with `Content-Disposition: attachment`.

### `GET /api/export/{run_id}/csv`

Generate a CSV of opportunities with columns:

```
tx_hash,block_number,timestamp,strategy,gross_revenue,gas_cost,flash_loan_fee,builder_tip,net_profit,result,token_pair,dex_path
```

---

## 10. CORS Configuration

```rust
use tower_http::cors::{CorsLayer, Any};

let cors = CorsLayer::new()
    .allow_origin("http://localhost:8080")
    .allow_methods(Any)
    .allow_headers(Any);
```

---

## 11. Strategy Status Reference

| UI Strategy | Rust Implementation | Notes |
|-------------|-------------------|-------|
| `arb` | `TwoHopArb` + `MultiHopArb` | Combined as "arb" for UI |
| `jit` | `Jit` | |
| `jitarb` | `JitArb` | |
| `sandwich` | `Sandwich` | |
| `longtail` | ❌ Not implemented | UI has it off by default; skip gracefully |
| `aggregator` | ❌ Not implemented | Skip gracefully |

The API should silently skip unimplemented strategies and return an informational log entry.

---

## 12. Errors & Edge Cases

| Scenario | HTTP | Response |
|----------|------|----------|
| Run not found | `404` | `{"error": "run not found"}` |
| Invalid chain | `400` | `{"error": "unknown chain. Supported: ..."}` |
| Invalid block range | `400` | `{"error": "invalid block range"}` |
| RPC connection failed during run | `200` (SSE error event) | `event: error` sent, status = Error |
| Simulation cancelled | `200` | Status = Cancelled |

---

## 13. Implementation Order

1. **Scaffold `mev-backtest-api` crate** — axum server, CORS, health check
2. **`GET /api/chains`** — static chain metadata response
3. **Aggregation module** in `mev-backtest-core/src/aggregate.rs`
4. **`POST /api/simulate`** — parse request, spawn background task
5. **SSE pipeline streaming** — wire `BacktestRunner` to emit progress events per stage
6. **Data mapping** — `MevOpportunity` → `UiOpportunity` with trace builder
7. **`GET /api/simulate/{id}/results`** — return mapped results + analytics
8. **History endpoints** — save/load/delete result files
9. **Export endpoints** — JSON download, CSV generator
10. **UI integration** — update `simulationStore.ts` to replace mock data with API calls

---

## 14. File Checklist

```
mev-bot-backtest/
├── mev-backtest-api/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs              # Axum server, routes, CORS
│       ├── routes/
│       │   ├── mod.rs
│       │   ├── chains.rs        # GET /api/chains
│       │   ├── simulate.rs      # POST /api/simulate
│       │   ├── status.rs        # GET /api/simulate/{id}/status (SSE)
│       │   ├── results.rs       # GET /api/simulate/{id}/results
│       │   ├── history.rs       # GET/DELETE /api/history
│       │   └── export.rs        # GET /api/export/{id}/json|csv
│       ├── state.rs             # RunState, RunManager
│       ├── sse.rs               # SSE event types + streaming
│       └── mapping.rs           # MevOpportunity -> UiOpportunity transform + trace builder
├── mev-backtest-core/
│   └── src/
│       ├── aggregate.rs         # NEW: SummaryMetrics, StrategyMetrics, DexMetrics
│       └── lib.rs               # Add `pub mod aggregate;`
└── Cargo.toml                   # Add mev-backtest-api to workspace members
```
