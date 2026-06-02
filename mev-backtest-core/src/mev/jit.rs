use crate::mev::opportunity::MevOpportunity;
use crate::pool::state::PoolManager;

/// JIT (Just-In-Time) liquidity detection stub.
/// Not yet implemented — logs a warning on first use.
pub struct JitDetector;

impl JitDetector {
    pub fn detect(
        &self,
        _pool_manager: &PoolManager,
        _block_number: u64,
        _tx_index: usize,
        _timestamp: u64,
    ) -> Vec<MevOpportunity> {
        tracing::warn!("JIT detection not yet implemented");
        Vec::new()
    }
}

/// JIT + Arbitrage combo detection stub.
/// Not yet implemented.
pub struct JitArbDetector;

impl JitArbDetector {
    pub fn detect(
        &self,
        _pool_manager: &PoolManager,
        _block_number: u64,
        _tx_index: usize,
        _timestamp: u64,
    ) -> Vec<MevOpportunity> {
        tracing::warn!("JIT+Arb detection not yet implemented");
        Vec::new()
    }
}
