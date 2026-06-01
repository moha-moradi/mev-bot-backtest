use crate::mev::opportunity::MevOpportunity;
use crate::mev::two_hop::TwoHopArbDetector;
use crate::pool::registry::PoolRegistry;
use crate::pool::state::{PoolManager, PoolState, UniswapV2PoolState};
use crate::replay::BlockReplayer;
use crate::resolver::ResolvedRange;
use crate::rpc::RpcClient;

/// Orchestrates backtesting: replays blocks and detects MEV opportunities.
pub struct BacktestRunner {
    replayer: BlockReplayer,
    pool_manager: PoolManager,
    detector: TwoHopArbDetector,
}

impl BacktestRunner {
    pub fn new(
        replayer: BlockReplayer,
        pool_manager: PoolManager,
        min_profit_usd: f64,
    ) -> Self {
        BacktestRunner {
            replayer,
            pool_manager,
            detector: TwoHopArbDetector::new(min_profit_usd),
        }
    }

    /// Initialize pool manager by loading registry and fetching on-chain reserves.
    pub async fn init_pools(
        pool_manager: &mut PoolManager,
        registry_path: Option<&str>,
        rpc: &RpcClient,
        block_num: u64,
    ) {
        let pool_infos = PoolRegistry::load_optional(registry_path);
        if pool_infos.is_empty() {
            tracing::warn!("No pools loaded from registry, skipping TwoHopArb detection");
            return;
        }

        tracing::info!("Loading {} pools from registry", pool_infos.len());

        for info in &pool_infos {
            if info.pool_type == "uniswap_v2" {
                pool_manager.add_pool(PoolState::UniswapV2(UniswapV2PoolState {
                    info: info.clone(),
                    reserve0: 0,
                    reserve1: 0,
                }));
            }
        }

        tracing::info!(
            "Initializing {} pool reserves at block {}",
            pool_manager.pool_count(),
            block_num
        );
        pool_manager.init_from_rpc(rpc, block_num).await;

        let initialized = pool_manager.initialized_count();
        tracing::info!(
            "{}/{} pools initialized",
            initialized,
            pool_manager.pool_count()
        );
    }

    /// Replay a single block using incremental replay and detect MEV after each tx.
    pub fn run_block(&mut self, block_num: u64) -> anyhow::Result<Vec<MevOpportunity>> {
        let txs = self.replayer.load_txs(block_num)?;
        if txs.is_empty() {
            return Ok(Vec::new());
        }

        let timestamp = txs.first().map(|_| {
            // Approximate: we could fetch block data for exact timestamp,
            // but for MVP we use the replayer's built-in block access
            0u64
        }).unwrap_or(0);

        let mut all_opportunities = Vec::new();
        let mut pool_manager = self.pool_manager.clone();
        let detector = &self.detector;

        self.replayer.replay_each(block_num, |i, tx, _db| {
            pool_manager.update_from_logs(&tx.logs);

            let opps = detector.detect(&pool_manager, block_num, i, timestamp);
            if !opps.is_empty() {
                tracing::info!(
                    "Block {} tx {}: {} arb opportunities",
                    block_num,
                    i,
                    opps.len()
                );
            }
            all_opportunities.extend(opps);

            Ok(())
        })?;

        self.pool_manager = pool_manager;
        Ok(all_opportunities)
    }

    /// Run backtest over a resolved block range.
    pub fn run_range(
        &mut self,
        resolved: &ResolvedRange,
    ) -> anyhow::Result<Vec<MevOpportunity>> {
        let mut all = Vec::new();
        for block_num in resolved.start_block..=resolved.end_block {
            match self.run_block(block_num) {
                Ok(opps) => {
                    tracing::info!(
                        "Block {} done: {} opportunities",
                        block_num,
                        opps.len()
                    );
                    all.extend(opps);
                }
                Err(e) => {
                    tracing::error!("Block {} failed: {:?}", block_num, e);
                }
            }
        }
        Ok(all)
    }
}


