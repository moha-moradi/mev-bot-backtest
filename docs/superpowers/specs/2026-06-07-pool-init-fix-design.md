# Pool State Initialization — Hybrid eth_call + eth_getStorageAt

## Motivation

Pool state initialization via `eth_call` (`getReserves()` for V2, `slot0()`/`liquidity()` for V3) works reliably on full archive nodes but fails for many pools when using constrained public endpoints like Infura free tier. On Polygon with Infura, ~80% of V2 pool `getReserves()` calls revert even though the pools are active and well-known.

`eth_getStorageAt` reads raw storage slots directly — it does not execute contract code — making it more resilient on constrained archive nodes. This spec hybrids the two approaches: try `eth_call` first, fall back to `eth_getStorageAt` on failure.

## Design

### Try-call-fallback pattern

Both `fetch_v2_reserves` and `fetch_v3_state` wrap their existing `eth_call` in a try-first / fallback-second pattern:

```text
fn fetch_v2_reserves(rpc, pool, block):
    result = rpc.call(pool, getReserves(), block)
    if result is Ok:
        return decode_abi(result)
    # fallback: eth_getStorageAt
    trace("falling back to storage slot for {pool}")
    raw = rpc.get_storage_at(pool, slot=6, block)
    return decode_storage(raw)
```

### V2 storage layout (slot 6)

Uniswap V2 pairs store reserves as a packed struct at **slot 6**.
Solidity packs struct members from the least significant byte (LSB).

```
struct: uint112 reserve0 | uint112 reserve1 | uint32 blockTimestampLast
```

In big-endian bytes (byte[0] = MSB, byte[31] = LSB):
- Bytes 18-31 (14 bytes): `reserve0` (lowest 112 bits)
- Bytes 4-17 (14 bytes): `reserve1` (middle 112 bits)
- Bytes 0-3 (4 bytes): `blockTimestampLast` (highest 32 bits)

Decoding:
```
r0 = u128.from_be_bytes(raw[18..32])       // low 112 bits
r1 = u128.from_be_bytes(raw[4..18])        // middle 112 bits
ts = u32.from_be_bytes(raw[0..4])          // high 32 bits
```

### V3 storage layout (slot 0 + slot 1)

Uniswap V3 pool stores its global state at:

- **Slot 0** — struct packed from LSB: `sqrtPriceX96 (uint160, 20 bytes | bits 0..159) | tick (int24, 3 bytes | bits 160..183) | observationIndex (uint16) | observationCardinality (uint16) | observationCardinalityNext (uint16) | feeProtocol (uint8) | unlocked (bool)`
  - Bytes 12-31 (big-endian): `sqrtPriceX96` (lowest 160 bits)
  - Bytes 9-11: `tick` (3 bytes as int24, sign-extended to i32)
- **Slot 1** — `liquidity (uint128)` — lower 128 bits of the slot

Decoding:
```
sqrt_price_x96 = U256.from_be_bytes(raw[12..32])      // low 160 bits as U256
tick = i32.from_be_bytes([sign_byte, raw[9], raw[10], raw[11]])
liquidity = u128.from_be_bytes(raw[16..32])           // lower 128 bits of slot 1
```

### Code changes

**File:** `mev-backtest-core/src/pool/state.rs`

| Function | Change |
|---|---|
| `fetch_v2_reserves` | Wrap body: try `eth_call` → `?`. On `None`, call `fetch_v2_reserves_storage()`. |
| `fetch_v2_reserves_storage` (new) | Calls `rpc.get_storage_at(pool, U256::from(6), block)`. Decodes packed reserves. |
| `fetch_v3_state` | Wrap body: try `eth_call` chain → `?`. On `None`, call `fetch_v3_state_storage()`. |
| `fetch_v3_state_storage` (new) | Calls `rpc.get_storage_at(pool, U256::from(0), block)` and `rpc.get_storage_at(pool, U256::from(1), block)`. Decodes slot0 and liquidity. |

### No new dependencies

`RpcClient::get_storage_at()` already exists at `rpc.rs:223`.

### Testing

1. **Unit test**: `fetch_v2_reserves_storage` with known byte patterns verifies correct reserve decoding.
2. **Integration test**: `test_init_via_storage_at` — run against a real (or mocked) block, confirm fallback decodes correctly.
3. **Existing tests**: all existing `init_from_rpc` tests continue to pass (they use the `eth_call` path unchanged).

## Risks

- Storage layout could differ for non-standard V2/V3 forks (e.g., pancakeswap V2 uses same layout as Uniswap V2). If a fork changes storage layout, the storage path would produce garbage silently. Mitigation: the `eth_call` path is always tried first and works for standard contracts.
- `eth_getStorageAt` rate limits may differ from `eth_call` limits. No evidence of this being worse; it's a lighter operation for the node.
