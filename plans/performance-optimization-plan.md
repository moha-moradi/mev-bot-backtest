# Performance Optimization Plan — Scaling to 30 Days of Polygon

## Bottleneck Analysis

| Phase | Current design | Est. cost (1.3M blocks) | % of total |
|-------|---------------|------------------------|------------|
| RPC fetch | 20 concurrent req, per-block eth_getBlock + eth_getReceipts | ~3.5 hr | 10% |
| Sled I/O | Single-tree LSM, no compaction tuning, bincode serde per access | ~2 hr | 6% |
| EVM replay (sequential) | `run_range()` iterates blocks 1-by-1, fresh EVM context per block | ~24-36 hr | 80% |
| Pool state init | `init_pools()` fetches reserves sequentially for all pools | ~0.5 hr | 1% |
| RPC fallback (replay) | `CachedRpcDb` makes `eth_getProof` / `getStorageAt` on cache miss | ~1 hr | 3% |

**Dominant term: sequential block replay.** Everything else is secondary.

---

## Phase 1: Profile First (hotpath.rs)

Before any optimization, establish baselines:

```bash
# Install hotpath
cargo install hotpath

# Profile replay of a single block (micro-benchmark)
hotpath record -- cargo test --release replay_single_block -- --nocapture
hotpath report replay_single_block --flamegraph

# Profile run_range for 100 blocks (meso-benchmark)
hotpath record -- cargo run --release -- run --blocks 100 --chain polygon
hotpath report run_100_blocks --flamegraph

# Profile fetch for 1000 blocks
hotpath record -- cargo run --release -- fetch --blocks 1000 --chain polygon
hotpath report fetch_1000_blocks --flamegraph
```

**Hotpath integration** (`core/Cargo.toml`):
```toml
[dev-dependencies]
hotpath = "0.1"  # or latest
```

Add a benchmark suite:
```rust
#[cfg(test)]
mod benchmarks {
    use hotpath::bench;
    // ... benchmark replay_single_block, filter_fast_path, etc.
}
```

**What to measure:** Instructions retired, cache misses, branch mispredictions, syscall count per block.

---

## Phase 2: Parallel Block Replay (Rayon)

**Biggest win.** `run_range()` at `core/src/run.rs:270-291` is a sequential `for` loop over blocks. Each block is independent — no shared mutable state across blocks. This is embarrassingly parallel.

### Design

Replace `run_range()` with parallel iteration:

```rust
// core/src/run.rs
use rayon::prelude::*;

pub fn run_range_par(
    &mut self,
    resolved: &ResolvedRange,
    thread_count: usize,
) -> anyhow::Result<Vec<MevOpportunity>> {
    let pool_manager = std::mem::take(&mut self.pool_manager);
    
    let opps: Vec<Vec<MevOpportunity>> = (resolved.start_block..=resolved.end_block)
        .collect::<Vec<_>>()
        .par_iter()
        .with_max_len(thread_count)
        .map(|&block_num| {
            // Each thread gets its own PoolManager cloned from the base state
            let mut local_runner = self.clone_for_block(block_num, &pool_manager);
            local_runner.run_block(block_num).unwrap_or_default()
        })
        .collect();
    
    Ok(opps.into_iter().flatten().collect())
}
```

### Requirements

1. **Make `BacktestRunner` clonable** — currently owns `replayer: BlockReplayer` and `pool_manager: PoolManager`. `BlockReplayer` contains `CacheStore` + `RpcClient` (both `Clone`), and `tokio::runtime::Handle` (`Clone`). `PoolManager` needs `Clone`.

2. **Make `CachedRpcDb` thread-safe** — currently uses `&mut self` via `Database` trait. Rayon needs `Send + Sync`. Options:
   - Wrap in `Mutex` — acceptable since per-block EVM is single-threaded
   - Or use `DatabaseRef` + `CacheDB::new_with_ref` instead

3. **Pool state forking** — Each parallel block needs its own `PoolManager` initialized at the correct block number. Current design forwards pool state from block N to N+1 sequentially. For parallel mode, each block must independently fetch reserve state.

### Pool State Forking Strategy

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| **A. Snapshot fetch** | Each parallel task calls `init_pools()` independently at its block number | Correct, simple | N+ RPC calls per pool |
| **B. Reserve history cache** | Cache `getReserves()` results in sled keyed `reserves:{chain}:{pool}:{block}` | Reuse across runs | More sled writes |
| **C. Sequential pool forward** | Run pool state forward from earliest block in range, snapshotting at intervals | No extra RPC calls | Complex, memory-heavy |

**Recommendation:** Start with Option A (simplest, correct). Optimize with Option B if pool init becomes a bottleneck.

### Thread count heuristic

```rust
fn optimal_thread_count(range_size: u64) -> usize {
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    // Don't oversubscribe; each thread has significant memory (EVM context)
    cores.min(8)  // 8 threads = ~8 GB memory for EVM contexts
}
```

### Expected speedup

| Cores | Speedup | Est. time (1.3M blocks) |
|-------|---------|------------------------|
| 1 (current) | 1× | 30 hr |
| 4 | 3.5× | 8.5 hr |
| 8 | 6× | 5 hr |
| 16 | 9× | 3.3 hr |

Amdahl's law: the serial fraction (pool init, result merge) is ~2%, so scaling is near-linear.

---

## Phase 3: Smart Batched RPC Fetch

### Adaptive Concurrency

Current: fixed `Semaphore` with `min(parallelism, 20)` (`core/src/fetch.rs:59`).

Replace with adaptive token bucket that adjusts concurrency based on RPC latency:

```rust
// core/src/fetch.rs
pub struct AdaptiveFetcher {
    base_concurrency: usize,
    max_concurrency: usize,
    latency_window: VecDeque<Duration>,
    error_rate: f64,
}

impl AdaptiveFetcher {
    fn adjust_concurrency(&mut self, latency: Duration, success: bool) {
        let window = 100;
        self.latency_window.push_back(latency);
        if self.latency_window.len() > window {
            self.latency_window.pop_front();
        }
        let avg_latency = self.latency_window.iter().sum::<Duration>() / self.latency_window.len() as u32;
        // Back off if latency > 500ms (RPC is overloaded)
        // Ramp up if latency < 200ms and error_rate < 1%
    }
}
```

### Batch receipt fetching

Current: one `eth_getBlockReceipts` call per block. Receipts for a block range can be batched into fewer RPC calls.

Some RPCs support batch JSON-RPC:
```rust
// Batch N receipt requests into one HTTP call
let batch: Vec<JsonRpcRequest> = blocks.iter()
    .map(|b| JsonRpcRequest::new("eth_getBlockReceipts", vec![to_hex(*b)]))
    .collect();
let responses = rpc.batch_call(batch).await?;
```

### Pre-fetch pipelining

Overlap fetch and discovery: while fetching block N, already have block N+1 in-flight. Tokio already does this via `try_join_all`, but ensure the semaphore isn't causing head-of-line blocking.

---

## Phase 4: Sled → Replace with SQLite (or RocksDB)

Sled is the wrong tool at >10 GB:
- No incremental compaction control
- Single-tree bottleneck (all keys merged)
- Memory-mapped BTree — performance cliff at high write volume
- No concurrent reader isolation

### Option A: SQLite via `rusqlite` (recommended)

```toml
# core/Cargo.toml
rusqlite = { version = "0.31", features = ["bundled", "vtab"] }
```

Schema:
```sql
CREATE TABLE IF NOT EXISTS blocks (
    chain_id INTEGER NOT NULL,
    block_num INTEGER NOT NULL,
    data BLOB NOT NULL, -- bincode BlockData
    PRIMARY KEY (chain_id, block_num)
);

CREATE TABLE IF NOT EXISTS txs (
    chain_id INTEGER NOT NULL,
    block_num INTEGER NOT NULL,
    data BLOB NOT NULL,
    PRIMARY KEY (chain_id, block_num)
);

CREATE TABLE IF NOT EXISTS receipts (
    chain_id INTEGER NOT NULL,
    block_num INTEGER NOT NULL,
    data BLOB NOT NULL,
    PRIMARY KEY (chain_id, block_num)
);

CREATE TABLE IF NOT EXISTS accounts (
    chain_id INTEGER NOT NULL,
    block_num INTEGER NOT NULL,
    address BLOB NOT NULL,
    nonce INTEGER,
    balance BLOB,
    code_hash BLOB,
    PRIMARY KEY (chain_id, block_num, address)
);

CREATE TABLE IF NOT EXISTS slots (
    chain_id INTEGER NOT NULL,
    block_num INTEGER NOT NULL,
    address BLOB NOT NULL,
    slot BLOB NOT NULL,
    value BLOB NOT NULL,
    PRIMARY KEY (chain_id, block_num, address, slot)
);

CREATE TABLE IF NOT EXISTS codes (
    chain_id INTEGER NOT NULL,
    address BLOB NOT NULL,
    data BLOB NOT NULL,
    PRIMARY KEY (chain_id, address)
);

CREATE TABLE IF NOT EXISTS manifests (
    run_id TEXT PRIMARY KEY,
    chain TEXT NOT NULL,
    start_block INTEGER NOT NULL,
    end_block INTEGER NOT NULL,
    resolved_at INTEGER NOT NULL,
    range_mode TEXT NOT NULL,
    strategies TEXT NOT NULL,
    flash_loan_provider TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS discovered_pools (
    chain_id INTEGER NOT NULL,
    address BLOB NOT NULL,
    token0 BLOB NOT NULL,
    token1 BLOB NOT NULL,
    fee INTEGER NOT NULL,
    name TEXT,
    dex_type INTEGER NOT NULL,
    tick_spacing INTEGER,
    PRIMARY KEY (chain_id, address)
);

CREATE TABLE IF NOT EXISTS discovery_cursors (
    chain_id INTEGER NOT NULL,
    factory BLOB NOT NULL,
    block_num INTEGER NOT NULL,
    PRIMARY KEY (chain_id, factory)
);
```

Benefits:
- **Indexed** — O(log n) lookups vs sled's scan-based iteration
- **Transactions** — atomic batch writes
- **Concurrent readers** — WAL mode allows reads during writes
- **Size** — SQLite with BLOBs is typically 10-30% smaller than sled's LSM overhead
- **Portability** — single `.sqlite` file, easy inspection with CLI tools
- **No compaction tuning needed** — VACUUM is simple `PRAGMA auto_vacuum=INCREMENTAL`

### Option B: RocksDB via `rust-rocksdb`

Benefits:
- Better write throughput than SQLite
- Native column families (tables are free)
- Built-in compression (zstd/lz4)

Downside:
- Heavy compilation (C++ rocksdb), larger binary
- More tuning surface (memtable size, write buffer, compaction style)

### Migration Strategy

1. New trait `Database` (abstract over sled/SQLite/RocksDB):

```rust
// core/src/storage/mod.rs
pub trait CacheBackend: Send + Sync + Clone {
    fn get_block(&self, chain_id: u64, block_num: u64) -> Result<Option<BlockData>>;
    fn put_block(&self, chain_id: u64, block_num: u64, block: &BlockData) -> Result<()>;
    // ... all 9 data families
}

pub struct SqliteBackend { /* rusqlite::Connection in a r2d2 pool */ }
pub struct SledBackend { /* current sled::Db */ }
```

2. `CacheStore` becomes generic: `CacheStore<B: CacheBackend>`
3. Default: `SqliteBackend`; keep `SledBackend` as backward compat for existing caches
4. Config key: `cache_backend = "sqlite" | "sled"`

---

## Phase 5: EVM Replay Micro-Optimizations

### 5a. CachePool for `init_pools()`

`PoolManager::init_from_rpc()` (`core/src/run.rs:93`) calls RPC per pool. Cache results in memory or sled:

```rust
// core/src/pool/state.rs
pub struct CachedPoolReserves {
    cache: CacheStore,
    chain_id: u64,
    block_cache: LruCache<(Address, u64), (u128, u128)>,  // (pool, block) -> reserves
}
```

### 5b. Lazy code loading during replay

`CachedRpcDb::basic()` (`core/src/replay.rs:176`) calls `self.rpc.get_proof()` on first access to an address. This fetches nonce, balance, code_hash in one `eth_getProof` call. But for EOA accounts (externally owned), `code_hash` is always `KECCAK_EMPTY`. Skip the `eth_getCode` call for EOA by checking the first byte of the code_hash:

```rust
// In basic(): after get_proof, if code_hash == KECCAK_EMPTY, skip get_code
```

Saves one RPC call per EOA accessed.

### 5c. Batch storage reads

`CachedRpcDb::storage()` (line 264) fetches one slot at a time. revm often reads multiple slots for the same address (e.g., pair reserves at slots 8 and 9). Batch them:

```rust
pub fn storage_batch(&mut self, address: Address, indices: &[U256]) -> Result<Vec<U256>, Self::Error> {
    let mut results = Vec::with_capacity(indices.len());
    let mut uncached = Vec::new();
    for &index in indices {
        if let Some(value) = self.storage.get(&(address, index)) {
            results.push(*value);
        } else if let Some(value) = self.cache.get_slot(self.block_number, address, index)? {
            results.push(value);
            self.storage.insert((address, index), value);
        } else {
            uncached.push(index);
        }
    }
    if !uncached.is_empty() {
        // Batch RPC: call eth_getStorageAt for all uncached slots at once
        let fetched = self.rpc.get_storage_batch(address, &uncached, self.block_number)?;
        for (&index, value) in uncached.iter().zip(fetched) {
            self.storage.insert((address, index), value);
            self.cache.put_slot(self.block_number, address, index, value);
            results.push(value);
        }
    }
    Ok(results)
}
```

### 5d. Payment and basefee contract filters

Polygon has system contracts (`0x1001`, `0x1010` at `replay.rs:612-614`) that add logs but are irrelevant for MEV detection. Already filtered in receipt verification — also skip them in the EVM filter to avoid unnecessary `eth_getProof` calls during replay:

```rust
fn is_polygon_system_addr(addr: &Address) -> bool {
    SYSTEM_ADDRS.contains(addr)
}
```

Add to `replay_each_filtered` filter closure (`run.rs:161-168`).

---

## Phase 6: Incremental/Resumable Backtests

### Checkpoint every N blocks

Serialize pool state + last processed block to sled/SQLite periodically:

```rust
// core/src/run.rs
pub struct Checkpoint {
    block_num: u64,
    pool_manager: PoolManager, // serializable
    progress: f64,
    etc: u64,
}

impl BacktestRunner {
    fn save_checkpoint(&self, block_num: u64) {
        // Write to cache DB or separate checkpoint file
    }
    
    fn load_checkpoint(&mut self, block_num: u64) -> Option<()> {
        // Resume from checkpoint
    }
}
```

When interrupted, resume from last checkpoint instead of starting over.

### Run manifest already exists

`RunManifest` (`core/src/cache.rs:27-36`) stores run metadata. Extend it with `checkpoint_block: Option<u64>` for resume support.

---

## Phase 7: Implementation Order & Effort Estimate

| # | Task | Effort | Speedup | Risk |
|---|------|--------|---------|------|
| 1 | Profile with hotpath.rs + flamegraph | 1 day | — | Low (measurement only) |
| 2 | Parallel block replay (Rayon) | 3 days | **6-8×** | Medium (thread safety) |
| 3 | Adaptive RPC fetch concurrency | 1 day | 1.2× | Low |
| 4 | Storage batch reads (CachedRpcDb) | 1 day | 1.3× | Low |
| 5 | Sled → SQLite migration | 5 days | 1.5-2× | Medium (schema, migration) |
| 6 | Pool reserves cache | 1 day | 1.1× | Low |
| 7 | Incremental checkpoints | 2 days | N/A (UX) | Low |
| 8 | EOA skip optimization | 0.5 day | 1.1× | Low |

**Total:** ~14 days for 8-12× throughput improvement.

**Recommended sprint order:** 1 → 2 → 3 → 4 → 7 → 5 → 6 → 8

---

## File Change Summary

| File | Change |
|------|--------|
| `core/Cargo.toml` | Add `rayon`, `hotpath` (dev), `lru`, `rusqlite` (optional) |
| `core/src/run.rs` | Add `run_range_par()`, `clone_for_block()`, checkpoint support |
| `core/src/replay.rs` | Make `CachedRpcDb` thread-safe, add `storage_batch()`, EOA skip |
| `core/src/fetch.rs` | Adaptive concurrency, batch JSON-RPC for receipts |
| `core/src/cache.rs` | Abstract to `CacheBackend` trait, SQLite impl |
| `core/src/storage/` | New module: `mod.rs`, `sqlite.rs`, `sled.rs` |
| `core/src/pool/state.rs` | Add `CachedPoolReserves`, `PoolManager::clone()` |
| `core/src/config.rs` | Add `cache_backend` config field |
| `core/src/lib.rs` | Add `pub mod storage;` |
| `cli/src/main.rs` | Rayon thread pool init |
