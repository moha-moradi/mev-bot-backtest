# MultiHopArb Detection — Design Spec

## Overview
Generalize TwoHopArb (2-pool arbitrage) to N-pool paths by discovering token-graph paths and composing quoting functions for ternary search optimization.

## Strategy: `MultiHopArb`

Already defined in `Strategy` enum in `types.rs`. Connects to existing `Strategy::all()`, `FromStr`, `Display`.

## MevOpportunity Extension

Add an optional `path` field to capture the full pool sequence:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub path: Option<Vec<Address>>,
```

- MultiHopArb sets `path = Some(full_path)`, `pool_a = path[0]`, `pool_b = path[last]`
- TwoHopArb and all other strategies leave `path = None` (no serialization impact)

## Path Finding

Algorithm: BFS-limited walk of the token-pool bipartite graph.

- Input: `PoolManager`, `max_depth` (default 4)
- Start from every pool; for each, walk to adjacent pools sharing a token
- Collect all paths of length 2..=max_depth with unique pool sequences
- No duplicate pools in a path (cycle prevention)
- Filtered to paths where profit > 0 after ternary search

Pseudo:
```
for each start_pool in pm.all_pools():
    for each direction (token_in, token_out):
        BFS(queue: (path, token_in, token_out))
            if path.len() >= 2: emit path
            if path.len() >= max_depth: skip
            for next_pool in pm.pools_for_token(token_out):
                if next_pool not in path:
                    (_, next_out) = other_token(next_pool, token_out)
                    push (path + next_pool, token_in, next_out)
```

## Quoting & Optimization

### Composed Quote Function
```
chain_quote(path, pm, amount):
    current = amount
    for each window (pool_a, pool_b) in path.windows(2):
        shared = shared_token(pool_a, pool_b)
        current = single_step_quote(pool_a, pool_b, shared, current)
    return current
```

Bridge `single_step_quote` reuses `quote_path` logic internally — quotes the pair `(pool_a, pool_b)` with `shared_token` in the [sell pool] direction.

### Optimization Algorithm

Use ternary search over `[0, max_input]` to maximize `profit(x) = output_amount(x) - input_amount(x)`.

The existing `optimal_two_hop_arb_generic` already implements ternary search with a closure-based quoting function. We extract the ternary search loop into a standalone `ternary_search_maximize(fn, lo, hi, iterations) -> u128` that can be reused.

```
optimal_nhop(quote_fn, max_input):
    return ternary_search_maximize(|x| quote_fn(x) - x, 0, max_input, 80)
```

If `profit <= 0`, skip (no opportunity).

### Max Input Determination

- V2 pool: `min(reserve0, reserve1)` (same as TwoHopArb)
- V3 pool: `liquidity` (same as TwoHopArb: `max(a.liquidity, b.liquidity)`)
- Chain: min of all individual pool max inputs

## Gas Cost

We reuse `GasConfig::compute_gas_cost()` but scale the gas limit by path length:

```rust
let gas_cost_wei = GasConfig {
    gas_limit: self.gas_config.gas_limit * path.len() as u64,
    ..self.gas_config
}.compute_gas_cost(base_fee_per_gas);
```

This accounts for N swaps in the arb. No new config fields.

## Integration with Runner

In `run.rs:on_tx` callback, after `TwoHopArbDetector::detect`:

```rust
let multi_opps = MultiHopArbDetector::detect(
    &pm, block_num, i, timestamp, base_fee_per_gas, self.gas_config,
);
all_opportunities.extend(multi_opps);
```

MultiHopArbDetector always runs alongside TwoHopArbDetector in the `on_tx` callback. No strategy filtering at the runner level — if the detector finds no profitable paths, it returns `Vec::new()`. The strategies list is for display/planning only.

## Module Structure

```
mev/
  mod.rs          — add `pub mod multi_hop`
  opportunity.rs  — add `path: Option<Vec<Address>>`
  two_hop.rs      — unchanged
  multi_hop.rs    — new: MultiHopArbDetector
```

## Edge Cases

- **No paths found** → empty Vec
- **Single pool** → no paths (path length >= 2)
- **All paths unprofitable** → empty Vec
- **V3 pools with no ticks** → skip quoting (returns None from quote_v3_exact_in)
- **Gas cost > profit** → filtered out (profit < 0 check in ternary search)

## Testing

### Unit tests in `multi_hop.rs`
- Empty pool manager returns nothing
- Two-pool path produces same result as TwoHopArb (regression)
- Three-pool triangular arb with price imbalance
- Cycle prevention (same pool not repeated)
- Max depth enforcement (default 4)

### Integration tests in `tests/integration.rs`
- Synthetic 3-pool circuit detecting triangular arb
- Verify `strategy == Strategy::MultiHopArb`
- Verify `path` contains the correct sequence
- Verify gas cost scaled by path length

## Future: JitArb & Sandwich

Both will follow the same pattern:
- New detector file in `mev/`
- Stateless `detect()` method
- `run.rs` calls it alongside existing detectors
- Uses `Strategy::JitArb` / `Strategy::Sandwich`

Not part of this spec.
