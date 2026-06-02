use crate::mev::opportunity::MevOpportunity;
use crate::pool::state::PoolManager;

/// Sandwich attack detection stub.
/// Not yet implemented — logs a warning on first use.
pub struct SandwichDetector;

impl SandwichDetector {
    pub fn detect(
        &self,
        _pool_manager: &PoolManager,
        _block_number: u64,
        _tx_index: usize,
        _timestamp: u64,
    ) -> Vec<MevOpportunity> {
        tracing::warn!("Sandwich detection not yet implemented");
        Vec::new()
    }
}
