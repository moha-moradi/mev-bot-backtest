# Pool Init Fix — Hybrid eth_call + eth_getStorageAt Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `eth_getStorageAt` fallback to pool state initialization so pools that fail `eth_call` on constrained archive nodes still get initialized.

**Architecture:** Two new helper functions (`fetch_v2_reserves_storage`, `fetch_v3_state_storage`) wrapping `RpcClient::get_storage_at()`. Existing `fetch_v2_reserves` / `fetch_v3_state` are modified to try the current `eth_call` path first, then fall back to storage on `None`.

**Tech Stack:** Rust, alloy U256, existing `RpcClient`

---

### Task 1: Add V2 storage fallback function

**Files:**
- Modify: `mev-backtest-core/src/pool/state.rs:414-428`

- [ ] **Step 1: Add `fetch_v2_reserves_storage`** after line 428

Add this function at the end of the `impl PoolManager` block (after the current `fetch_v2_reserves` function):

```rust
    /// Fallback: fetch V2 reserves via eth_getStorageAt slot 6.
    /// Slot 6 packs: uint112 reserve0 | uint112 reserve1 | uint32 blockTimestampLast
    /// (packed from LSB by Solidity). In big-endian bytes:
    ///   bytes[18..32] = reserve0 (14 bytes right-aligned to u128)
    ///   bytes[4..18]  = reserve1 (14 bytes right-aligned to u128)
    async fn fetch_v2_reserves_storage(
        rpc: &RpcClient,
        pool: Address,
        block: u64,
    ) -> Option<(u128, u128)> {
        let raw = rpc.get_storage_at(pool, U256::from(6), block).await.ok()?;
        let bytes = raw.to_be_bytes::<32>();
        let r0 = u128::from_be_bytes({
            let mut buf = [0u8; 16];
            buf[2..16].copy_from_slice(&bytes[18..32]);
            buf
        });
        let r1 = u128::from_be_bytes({
            let mut buf = [0u8; 16];
            buf[2..16].copy_from_slice(&bytes[4..18]);
            buf
        });
        Some((r0, r1))
    }
```

- [ ] **Step 2: Add V3 storage fallback functions** after the V3 `fetch_v3_state` function

Add after line 460:

```rust
    /// Fallback: fetch V3 state via eth_getStorageAt.
    /// Slot 0 packs (from LSB):
    ///   sqrtPriceX96 (uint160, 20 bytes | bits 0..159)
    ///   tick (int24, 3 bytes | bits 160..183)
    ///   + observationIndex/ cardinality/ feeProtocol/ unlocked (bits 184..247)
    /// In big-endian bytes: bytes[12..32] = sqrtPriceX96, bytes[9..12] = tick
    /// Slot 1: liquidity (uint128, bits 0..127), bytes[16..32] in big-endian
    async fn fetch_v3_state_storage(
        rpc: &RpcClient,
        pool: Address,
        block: u64,
    ) -> Option<(U256, i32, u128)> {
        // --- slot 0: sqrtPriceX96 + tick ---
        let slot0_raw = rpc.get_storage_at(pool, U256::ZERO, block).await.ok()?;
        let bytes = slot0_raw.to_be_bytes::<32>();
        // sqrtPriceX96: bytes[12..32] right-aligned within the lower 160 bits
        let sqrt_price_x96 = U256::from_be_bytes({
            let mut buf = [0u8; 32];
            buf[12..32].copy_from_slice(&bytes[12..32]);
            buf
        });
        // tick: bytes[9..12] as int24, sign-extended to i32
        let mut tick_buf = [0u8; 4];
        tick_buf[1..4].copy_from_slice(&bytes[9..12]);
        if tick_buf[1] & 0x80 != 0 {
            tick_buf[0] = 0xFF;
        }
        let tick = i32::from_be_bytes(tick_buf);

        // --- slot 1: liquidity ---
        let slot1_raw = rpc.get_storage_at(pool, U256::from(1), block).await.ok()?;
        let bytes = slot1_raw.to_be_bytes::<32>();
        let liquidity = u128::from_be_bytes({
            let mut buf = [0u8; 16];
            buf.copy_from_slice(&bytes[16..32]);
            buf
        });

        Some((sqrt_price_x96, tick, liquidity))
    }
```

---

### Task 2: Wire fallback into existing fetch functions

**Files:**
- Modify: `mev-backtest-core/src/pool/state.rs:414-428` and `431-460`

- [ ] **Step 1: Modify `fetch_v2_reserves` to try eth_call first, fall back to storage**

Replace lines 414-428 with:

```rust
    async fn fetch_v2_reserves(rpc: &RpcClient, pool: Address, block: u64) -> Option<(u128, u128)> {
        // Try eth_call getReserves() first
        let data = Bytes::copy_from_slice(&GET_RESERVES_SELECTOR);
        if let Ok(result) = rpc.call(pool, data, block).await {
            if result.len() >= 64 {
                let mut buf = [0u8; 32];
                buf.copy_from_slice(&result[..32]);
                let r0 = U256::from_be_bytes(buf).as_limbs()[0] as u128;
                buf.copy_from_slice(&result[32..64]);
                let r1 = U256::from_be_bytes(buf).as_limbs()[0] as u128;
                return Some((r0, r1));
            }
        }
        tracing::trace!("eth_call getReserves() failed, falling back to storage for {}", pool);
        // Fallback: direct storage read
        Self::fetch_v2_reserves_storage(rpc, pool, block).await
    }
```

- [ ] **Step 2: Modify `fetch_v3_state` to try eth_call first, fall back to storage**

Replace lines 431-460 with:

```rust
    /// Fetch V3 pool slot0() + liquidity() at a historical block.
    async fn fetch_v3_state(
        rpc: &RpcClient,
        pool: Address,
        block: u64,
    ) -> Option<(U256, i32, u128)> {
        // Try eth_call slot0() + liquidity() first
        let slot0_result = rpc.call(pool, V3_SLOT0_SELECTOR.clone(), block).await;
        let liq_result = rpc.call(pool, V3_LIQUIDITY_SELECTOR.clone(), block).await;
        if let (Ok(slot0), Ok(liq)) = (slot0_result, liq_result) {
            if slot0.len() >= 96 && liq.len() >= 32 {
                let mut buf = [0u8; 32];
                buf.copy_from_slice(&slot0[..32]);
                let sqrt_price_x96 = U256::from_be_bytes(buf);
                let mut tick_bytes = [0u8; 4];
                tick_bytes.copy_from_slice(&slot0[60..64]);
                let tick = i32::from_be_bytes(tick_bytes);
                buf.copy_from_slice(&liq[..32]);
                let liquidity = U256::from_be_bytes(buf).as_limbs()[0] as u128;
                return Some((sqrt_price_x96, tick, liquidity));
            }
        }
        tracing::trace!("eth_call slot0/liquidity() failed, falling back to storage for {}", pool);
        // Fallback: direct storage read
        Self::fetch_v3_state_storage(rpc, pool, block).await
    }
```

---

### Task 3: Build and test

**Files:**
- (none new, just verify)

- [ ] **Step 1: Build**

```bash
cargo build
```

Expected: clean build (only pre-existing proc-macro-error2 warning).

- [ ] **Step 2: Run unit tests**

```bash
cargo test --lib
```

Expected: 202 passed (existing tests exercise `fetch_v2_reserves` and `fetch_v3_state` — they use the eth_call path which works on local test RPC or mock).

- [ ] **Step 3: Run integration tests**

```bash
cargo test --test integration
```

Expected: 19 passed.

- [ ] **Step 4: Run clippy**

```bash
cargo clippy --all-targets
```

Expected: no new warnings.

- [ ] **Step 5: Commit**

```bash
git add mev-backtest-core/src/pool/state.rs
git commit -m "feat: fall back to eth_getStorageAt when eth_call fails for pool init"
```
