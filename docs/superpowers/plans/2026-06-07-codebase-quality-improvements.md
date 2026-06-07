# Codebase Quality Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate code duplication, add missing serde derives, add re-exports for ergonomics, and fix JIT detector sender inconsistency across the mev-bot-backtest codebase.

**Architecture:** The workspace has two crates: `mev-backtest-core` (library) and `mev-backtest-cli` (binary). All changes are in the core library. Each phase is self-contained and independently testable.

**Tech Stack:** Rust 2021 edition, serde, bincode, revm

---

## File Structure

### Files to create:
- `mev-backtest-core/src/utils.rs` — shared utility functions (extracted `u128_from_be_bytes`)

### Files to modify:
| File | Phase | Change |
|---|---|---|
| `mev-backtest-core/src/lib.rs` | 1 | Add `pub mod utils;` |
| `mev-backtest-core/src/pool/decoders.rs` | 1 | Replace private fn with shared import |
| `mev-backtest-core/src/pool/state.rs` | 1 | Replace private fn with shared import |
| `mev-backtest-core/src/mev/sandwich.rs` | 1 | Replace private fn with shared import |
| `mev-backtest-core/src/types.rs` | 2 | Add serde derives to `RangeMode` |
| `mev-backtest-core/src/pool/mod.rs` | 2 | Add `pub use` re-exports |
| `mev-backtest-core/src/mev/mod.rs` | 2 | Add `pub use` re-exports |
| `mev-backtest-core/src/mev/jit.rs` | 3 | Add sender check for burn matching, remove `#[allow(dead_code)]` |

---

## Phase 1: DRY Shared Utility

### Task 1.1: Create utils.rs with shared u128_from_be_bytes

**Files:**
- Create: `mev-backtest-core/src/utils.rs`
- Modify: `mev-backtest-core/src/lib.rs`

- [ ] **Step 1: Create `utils.rs`**

```rust
/// Decode a uint128 from the last 16 bytes of a byte slice.
/// If the slice is shorter than 16 bytes, leading bytes are treated as zero.
pub fn u128_from_be_bytes(bytes: &[u8]) -> u128 {
    let start = bytes.len().saturating_sub(16);
    let mut buf = [0u8; 16];
    buf.copy_from_slice(&bytes[start..start + 16]);
    u128::from_be_bytes(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u128_from_be_bytes_basic() {
        let mut buf = [0u8; 32];
        buf[16..32].copy_from_slice(&1000u128.to_be_bytes());
        assert_eq!(u128_from_be_bytes(&buf), 1000);
    }

    #[test]
    fn test_u128_from_be_bytes_zero() {
        let buf = [0u8; 32];
        assert_eq!(u128_from_be_bytes(&buf), 0);
    }

    #[test]
    fn test_u128_from_be_bytes_short_slice() {
        let buf = 42u128.to_be_bytes();
        assert_eq!(u128_from_be_bytes(&buf), 42);
    }
}
```

- [ ] **Step 2: Run test to verify it fails (no module declared yet)**

Run: `cd mev-backtest-core && cargo test utils::tests -q`
Expected: error[E0432] — module `utils` not declared in `lib.rs`

- [ ] **Step 3: Add module declaration to lib.rs**

In `mev-backtest-core/src/lib.rs`:

```rust
pub mod utils;
```

Insert after `pub mod types;` (keeping alphabetical order).

- [ ] **Step 4: Run test to verify it passes**

Run: `cd mev-backtest-core && cargo test utils::tests -q`
Expected: `running 3 tests ... ok`

- [ ] **Step 5: Commit**

```bash
git add mev-backtest-core/src/utils.rs mev-backtest-core/src/lib.rs
git commit -m "feat: add utils module with shared u128_from_be_bytes"
```

---

### Task 1.2: Replace duplicate in pool/decoders.rs

**Files:**
- Modify: `mev-backtest-core/src/pool/decoders.rs`

- [ ] **Step 1: Remove private fn and add import**

In `mev-backtest-core/src/pool/decoders.rs`:

1. Add import at the top after existing imports:
```rust
use crate::utils::u128_from_be_bytes;
```

2. **Remove** the private function (lines ~189-195):
```rust
// DELETE this block:
/// Decode a uint128 from the last 16 bytes of a 32-byte slice.
fn u128_from_be_bytes_32(bytes: &[u8]) -> u128 {
    let start = bytes.len().saturating_sub(16);
    let mut buf = [0u8; 16];
    buf.copy_from_slice(&bytes[start..start + 16]);
    u128::from_be_bytes(buf)
}
```

3. **Rename** all calls to `u128_from_be_bytes_32(` to `u128_from_be_bytes(` — 9 call sites total (lines 71, 118, 147, 148, 149, 150, 177, 178).

- [ ] **Step 2: Verify compilation and existing tests**

Run: `cd mev-backtest-core && cargo test pool::decoders::tests -q`
Expected: `running 5 tests ... ok`

- [ ] **Step 3: Commit**

```bash
git add mev-backtest-core/src/pool/decoders.rs
git commit -m "refactor: use shared u128_from_be_bytes in pool::decoders"
```

---

### Task 1.3: Replace duplicate in pool/state.rs

**Files:**
- Modify: `mev-backtest-core/src/pool/state.rs`

- [ ] **Step 1: Remove private fn and add import**

In `mev-backtest-core/src/pool/state.rs`:

1. Add import at the top with other crate imports:
```rust
use crate::utils::u128_from_be_bytes;
```

2. Update existing test import references — in the test module (around line 1066), **remove** the local `super::u128_from_be_bytes` test calls:
```rust
// In `#[cfg(test)] mod tests`:
// Replace:
    assert_eq!(super::u128_from_be_bytes(&buf), 1000);
// With:
    assert_eq!(crate::utils::u128_from_be_bytes(&buf), 1000);
```

3. **Remove** the private function (lines 678-683):
```rust
// DELETE this block:
fn u128_from_be_bytes(bytes: &[u8]) -> u128 {
    let mut buf = [0u8; 16];
    let start = bytes.len().saturating_sub(16);
    buf.copy_from_slice(&bytes[start..start + 16]);
    u128::from_be_bytes(buf)
}
```

4. **Remove** the test block in `state.rs` for `u128_from_be_bytes` (lines ~1066-1079), since those tests now live in `utils.rs`.

- [ ] **Step 2: Verify compilation and existing tests**

Run: `cd mev-backtest-core && cargo test pool::state::tests -q`
Expected: `running 39 tests ... ok`

- [ ] **Step 3: Commit**

```bash
git add mev-backtest-core/src/pool/state.rs
git commit -m "refactor: use shared u128_from_be_bytes in pool::state"
```

---

### Task 1.4: Replace duplicate in mev/sandwich.rs

**Files:**
- Modify: `mev-backtest-core/src/mev/sandwich.rs`

- [ ] **Step 1: Remove private fn and add import**

In `mev-backtest-core/src/mev/sandwich.rs`:

1. Add import at the top with existing imports:
```rust
use crate::utils::u128_from_be_bytes;
```

2. **Remove** the private function (lines ~156-161):
```rust
// DELETE this block:
fn u128_from_be_bytes_32(bytes: &[u8]) -> u128 {
    let start = bytes.len().saturating_sub(16);
    let mut buf = [0u8; 16];
    buf.copy_from_slice(&bytes[start..start + 16]);
    u128::from_be_bytes(buf)
}
```

3. **Rename** all calls to `u128_from_be_bytes_32(` to `u128_from_be_bytes(` — 4 call sites (lines 60, 61, 62, 63).

- [ ] **Step 2: Verify compilation and existing tests**

Run: `cd mev-backtest-core && cargo test mev::sandwich::tests -q`
Expected: `running 11 tests ... ok`

- [ ] **Step 3: Run full test suite to confirm no regressions**

Run: `cd mev-backtest-core && cargo test -q`
Expected: `running N tests ... ok` (all pass)

- [ ] **Step 4: Commit**

```bash
git add mev-backtest-core/src/mev/sandwich.rs
git commit -m "refactor: use shared u128_from_be_bytes in mev::sandwich"
```

---

## Phase 2: Module Consistency

### Task 2.1: Add serde derives to RangeMode

**Files:**
- Modify: `mev-backtest-core/src/types.rs`
- Verify: `mev-backtest-core/src/resolver.rs` (serialization of `mode` field in `ResolvedRange`)

- [ ] **Step 1: Add serde derives to RangeMode enum**

In `mev-backtest-core/src/types.rs` at line 205:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RangeMode {
    Days(u64),
    Blocks(u64),
    Single(u64),
    Range(u64, u64),
}
```

- [ ] **Step 2: Verify compilation**

Run: `cd mev-backtest-core && cargo build -q`
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add mev-backtest-core/src/types.rs
git commit -m "feat: add serde derives to RangeMode"
```

---

### Task 2.2: Add re-exports to pool/mod.rs

**Files:**
- Modify: `mev-backtest-core/src/pool/mod.rs`
- No test changes needed (import paths remain compatible)

- [ ] **Step 1: Add re-exports to pool/mod.rs**

Replace the content of `mev-backtest-core/src/pool/mod.rs`:

```rust
pub mod decoders;
pub mod dex_type;
pub mod discovery;
pub mod math;
pub mod registry;
pub mod state;
pub mod v3_quote;

pub use decoders::{V3SwapDecoded, V3MintBurnDecoded, CurveSwapDecoded, BalancerSwapDecoded};
pub use dex_type::DexType;
pub use discovery::DiscoveredPool;
pub use math::TwoHopArbResult;
pub use registry::PoolRegistry;
pub use state::{PoolInfo, PoolManager, PoolState, UniswapV2PoolState, UniswapV3PoolState, CurvePoolState, BalancerPoolState};
pub use v3_quote::quote_v3_exact_in;
```

- [ ] **Step 2: Verify compilation**

Run: `cd mev-backtest-core && cargo build -q`
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add mev-backtest-core/src/pool/mod.rs
git commit -m "feat: add re-exports to pool::mod for ergonomic imports"
```

---

### Task 2.3: Add re-exports to mev/mod.rs

**Files:**
- Modify: `mev-backtest-core/src/mev/mod.rs`

- [ ] **Step 1: Add re-exports to mev/mod.rs**

Replace the content of `mev-backtest-core/src/mev/mod.rs`:

```rust
pub mod jit;
pub mod jit_arb;
pub mod multi_hop;
pub mod opportunity;
pub mod sandwich;
pub mod two_hop;

pub use opportunity::{MevOpportunity, ResultsFile};
pub use sandwich::SandwichDetector;
pub use jit::JitDetector;
pub use jit_arb::JitArbDetector;
pub use multi_hop::MultiHopArbDetector;
pub use two_hop::TwoHopArbDetector;
```

- [ ] **Step 2: Verify compilation**

Run: `cd mev-backtest-core && cargo build -q`
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add mev-backtest-core/src/mev/mod.rs
git commit -m "feat: add re-exports to mev::mod for ergonomic imports"
```

---

## Phase 3: MEV Detector Consistency

### Task 3.1: Fix JIT burn sender matching

**Files:**
- Modify: `mev-backtest-core/src/mev/jit.rs`

**Context:** In `jit_arb.rs`, burn matching checks `mint.sender == sender` (line ~82). In `jit.rs`, no sender check is done — a burn from any address matches any mint on the same pool+tick range. This can produce false positives when two different LPs mint on the same pool in the same block.

- [ ] **Step 1: Add sender check to JIT burn matching**

In `mev-backtest-core/src/mev/jit.rs`, change the burn matching block (lines 96-107):

```rust
pub fn process_tx(
    &mut self,
    tx_index: usize,
    logs: &[ExecutedLog],
    sender: Option<Address>,
) {
    // ... (keep the event separation code unchanged) ...

    // Process Mint/Burn first (state changes)
    for (log, kind) in &mints_and_burns {
        let Some(decoded) = decode_v3_mint_burn(log) else { continue };
        match *kind {
            "mint" => {
                if decoded.amount > 0 {
                    self.active_mints
                        .entry(log.address)
                        .or_default()
                        .push(ActiveMint {
                            mint_tx_index: tx_index,
                            tick_lower: decoded.tick_lower,
                            tick_upper: decoded.tick_upper,
                            amount: decoded.amount as u128,
                            sender,
                            swapped: false,
                            burned: false,
                        });
                }
            }
            _ => {
                // Burn: find matching active mint on same pool + tick range + sender
                if let Some(mints) = self.active_mints.get_mut(&log.address) {
                    for mint in mints.iter_mut() {
                        if mint.burned { continue; }
                        if mint.tick_lower == decoded.tick_lower
                            && mint.tick_upper == decoded.tick_upper
                            && mint.sender == sender
                            && mint.mint_tx_index <= tx_index
                        {
                            mint.burned = true;
                            break;
                        }
                    }
                }
            }
        }
    }

    // Mark all active mints on swapped pools as swapped
    for log in &swaps {
        if let Some(mints) = self.active_mints.get_mut(&log.address) {
            for mint in mints.iter_mut() {
                mint.swapped = true;
            }
        }
    }
}
```

- [ ] **Step 2: Remove `#[allow(dead_code)]` from ActiveMint.sender**

In `mev-backtest-core/src/mev/jit.rs`, change `ActiveMint` struct:

```rust
struct ActiveMint {
    mint_tx_index: usize,
    tick_lower: i32,
    tick_upper: i32,
    amount: u128,
    sender: Option<Address>,
    swapped: bool,
    burned: bool,
}
```

Remove the `#[allow(dead_code)]` attribute above `sender`. The field is now actively read in burn matching.

- [ ] **Step 3: Verify compilation and tests**

Run: `cd mev-backtest-core && cargo test mev::jit::tests -q`
Expected: `running 7 tests ... ok`

- [ ] **Step 4: Run full test suite**

Run: `cd mev-backtest-core && cargo test -q`
Expected: all tests pass

- [ ] **Step 5: Commit**

```bash
git add mev-backtest-core/src/mev/jit.rs
git commit -m "fix: add sender check for JIT burn matching, consistent with JitArb"
```

---

## Self-Review Checklist

**1. Spec coverage:**
- Phase 1 addresses 3x duplication of `u128_from_be_bytes` ✓
- Phase 2 adds serde to `RangeMode` and re-exports to mod.rs files ✓
- Phase 3 fixes sender matching inconsistency between JIT and JitArb ✓

**2. Placeholder scan:** No "TBD", "TODO", or placeholder code found ✓

**3. Type consistency:**
- `u128_from_be_bytes` signature is `fn(&[u8]) -> u128` in all call sites ✓
- `ActiveMint.sender` is `Option<Address>` in both `jit.rs` and `jit_arb.rs` ✓
- `RangeMode` derives mirror other enum patterns in `types.rs` ✓
