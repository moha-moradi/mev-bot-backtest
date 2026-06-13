//! MEV detection strategies: JIT liquidity, sandwich attacks, arbitrage (two-hop, multi-hop, JIT arb).

pub mod jit;
pub mod jit_arb;
pub mod sandwich;
pub mod multi_hop;
pub mod opportunity;
pub mod two_hop;
