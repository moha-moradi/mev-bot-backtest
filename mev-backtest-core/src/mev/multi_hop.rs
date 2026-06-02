use crate::mev::opportunity::MevOpportunity;
use crate::pool::state::PoolManager;

/// Multi-hop arbitrage detection stub (>2 hops).
/// Not yet implemented — logs a warning on first use.
pub struct MultiHopDetector;

impl MultiHopDetector {
    pub fn detect(
        &self,
        _pool_manager: &PoolManager,
        _block_number: u64,
        _tx_index: usize,
        _timestamp: u64,
    ) -> Vec<MevOpportunity> {
        tracing::warn!("Multi-hop arbitrage detection not yet implemented");
        Vec::new()
    }
}
