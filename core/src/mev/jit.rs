//! JIT (just-in-time) liquidity detection — identifies liquidity added before a swap and removed after.

use std::collections::HashMap;
use alloy::primitives::{Address, U256};
use crate::data::ExecutedLog;
use crate::pool::decoders::{decode_v3_mint_burn, V3_SWAP_TOPIC, V3_MINT_TOPIC, V3_BURN_TOPIC};
use crate::mev::opportunity::MevOpportunity;
use crate::types::Strategy;

/// Tracks an active V3 Mint event that hasn't been fully processed.
#[derive(Debug, Clone)]
struct ActiveMint {
    mint_tx_index: usize,
    tick_lower: i32,
    tick_upper: i32,
    amount: u128,
    #[allow(dead_code)]
    sender: Option<Address>,
    swapped: bool,
    /// Has the corresponding Burn been seen for this specific position?
    burned: bool,
}

/// Detects Just-In-Time (JIT) liquidity provision on Uniswap V3.
///
/// Stateful per block: accumulates V3 events across sequential txs.
/// After each tx in block order, call `process_tx()` then `detect()`.
///
/// Patterns detected:
/// - **Full JIT:** Mint → Swap → Burn (complete cycle in one block)
/// - **Partial JIT:** Mint → Swap (liquidity deployed, swap traded against it,
///   but no burn detected within the block)
pub struct JitDetector {
    /// Pool address → active mints on that pool
    active_mints: HashMap<Address, Vec<ActiveMint>>,
    /// Track emitted mints by (pool, mint_tx_index, burned) to avoid duplicates
    emitted: Vec<(Address, usize, bool)>,
    /// Current block number
    block_number: u64,
}

impl JitDetector {
    pub fn new(block_number: u64) -> Self {
        JitDetector {
            active_mints: HashMap::new(),
            emitted: Vec::new(),
            block_number,
        }
    }

    /// Process a single transaction's logs and optional sender address.
    /// Call BEFORE `detect()` for each tx in block order.
    pub fn process_tx(
        &mut self,
        tx_index: usize,
        logs: &[ExecutedLog],
    sender: Option<Address>,
    ) {
        // Separate Mint/Burn from Swap events
        let mut mints_and_burns: Vec<(&ExecutedLog, &str)> = Vec::new();
        let mut swaps: Vec<&ExecutedLog> = Vec::new();

        for log in logs {
            if log.topics.is_empty() {
                continue;
            }
            let t0 = log.topics[0];
            if t0 == *V3_MINT_TOPIC || t0 == V3_BURN_TOPIC {
                let kind = if t0 == *V3_MINT_TOPIC { "mint" } else { "burn" };
                mints_and_burns.push((log, kind));
            } else if t0 == V3_SWAP_TOPIC {
                swaps.push(log);
            }
        }

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
                    // Burn: find matching active mint on same pool + tick range
                    if let Some(mints) = self.active_mints.get_mut(&log.address) {
                        for mint in mints.iter_mut() {
                            if mint.burned { continue; }
                            if mint.tick_lower == decoded.tick_lower
                                && mint.tick_upper == decoded.tick_upper
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

    /// Returns new JIT opportunities detected since the last call.
    /// Call AFTER `process_tx()` for each tx.
    pub fn detect(&mut self, timestamp: u64) -> Vec<MevOpportunity> {
        let mut opportunities = Vec::new();

        let pool_addrs: Vec<Address> = self.active_mints.keys().copied().collect();
        for pool in &pool_addrs {
            let Some(mints) = self.active_mints.get(pool) else { continue };
            for mint in mints {
                let dedup_key = (*pool, mint.mint_tx_index, mint.burned);
                if self.emitted.contains(&dedup_key) {
                    continue;
                }

                // Full JIT: Mint → Swap → Burn
                if mint.swapped && mint.burned {
                    self.emitted.push(dedup_key);
                    opportunities.push(Self::build_opp(
                        self.block_number, *pool, mint, timestamp, true,
                    ));
                // Partial JIT: Mint → Swap (no burn yet, or no burn in this block)
                } else if mint.swapped && !mint.burned {
                    self.emitted.push(dedup_key);
                    opportunities.push(Self::build_opp(
                        self.block_number, *pool, mint, timestamp, false,
                    ));
                }
            }
        }

        opportunities
    }

    fn build_opp(
        block_number: u64,
        pool: Address,
        mint: &ActiveMint,
        timestamp: u64,
        _burned: bool,
    ) -> MevOpportunity {
        // expected_profit = 0 for v1 (fee estimation requires complex on-chain math)
        // gas_cost_wei = 0 for v1 (JIT gas is incurred by the LP, not the detector)
        MevOpportunity {
            block_number,
            tx_index: mint.mint_tx_index,
            strategy: Strategy::Jit,
            pool_a: pool,
            pool_b: Address::ZERO,
            token_in: Address::ZERO,
            token_out: Address::ZERO,
            input_amount: U256::from(mint.amount),
            expected_profit: U256::ZERO,
            gas_cost_wei: 0,
            timestamp,
            path: None,
            tick_lower: Some(mint.tick_lower),
            tick_upper: Some(mint.tick_upper),
            liquidity_amount: Some(mint.amount),
            victim_tx_index: None,
            backrun_tx_index: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::{address, Bytes, B256};
    use crate::data::ExecutedLog;

    fn v3_mint_log(pool: Address, lower: i32, upper: i32, amount: u128) -> ExecutedLog {
        let mut data = Vec::new();
        let mut padded = [0u8; 32];
        padded[28..32].copy_from_slice(&lower.to_be_bytes());
        data.extend_from_slice(&padded);
        padded = [0u8; 32];
        padded[28..32].copy_from_slice(&upper.to_be_bytes());
        data.extend_from_slice(&padded);
        padded = [0u8; 32];
        padded[16..32].copy_from_slice(&amount.to_be_bytes());
        data.extend_from_slice(&padded);
        ExecutedLog {
            address: pool,
            topics: vec![*V3_MINT_TOPIC, B256::ZERO, B256::ZERO],
            data: data.into(),
        }
    }

    fn v3_burn_log(pool: Address, lower: i32, upper: i32, amount: u128) -> ExecutedLog {
        let mut data = Vec::new();
        let mut padded = [0u8; 32];
        padded[28..32].copy_from_slice(&lower.to_be_bytes());
        data.extend_from_slice(&padded);
        padded = [0u8; 32];
        padded[28..32].copy_from_slice(&upper.to_be_bytes());
        data.extend_from_slice(&padded);
        padded = [0u8; 32];
        padded[16..32].copy_from_slice(&amount.to_be_bytes());
        data.extend_from_slice(&padded);
        ExecutedLog {
            address: pool,
            topics: vec![V3_BURN_TOPIC, B256::ZERO, B256::ZERO],
            data: data.into(),
        }
    }

    fn v3_swap_log(pool: Address) -> ExecutedLog {
        ExecutedLog {
            address: pool,
            topics: vec![V3_SWAP_TOPIC, B256::ZERO, B256::ZERO],
            data: Bytes::from_static(&[0u8; 160]),
        }
    }

    fn pool_a() -> Address { address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa") }
    fn pool_b() -> Address { address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb") }

    #[test]
    fn test_empty_detector_returns_nothing() {
        let mut detector = JitDetector::new(1);
        let opps = detector.detect(100);
        assert!(opps.is_empty());
    }

    #[test]
    fn test_mint_swap_burn_detected() {
        let mut detector = JitDetector::new(1);

        // Tx 0: Mint on pool A
        detector.process_tx(0, &[v3_mint_log(pool_a(), -100, 100, 500_000)], None);
        assert!(detector.detect(100).is_empty(), "Mint alone is not JIT");

        // Tx 1: Swap on pool A
        detector.process_tx(1, &[v3_swap_log(pool_a())], None);
        let mut opps = detector.detect(100);
        assert_eq!(opps.len(), 1, "Mint+Swap should emit partial JIT");

        let opp = &opps[0];
        assert_eq!(opp.strategy, Strategy::Jit);
        assert_eq!(opp.pool_a, pool_a());
        assert_eq!(opp.tick_lower, Some(-100));
        assert_eq!(opp.tick_upper, Some(100));
        assert_eq!(opp.liquidity_amount, Some(500_000));

        // Tx 2: Burn matching the mint
        detector.process_tx(2, &[v3_burn_log(pool_a(), -100, 100, 500_000)], None);
        opps = detector.detect(100);
        assert_eq!(opps.len(), 1, "Burn should emit full JIT");
        assert_eq!(opps[0].tx_index, 0, "Should reference the mint tx index");
    }

    #[test]
    fn test_multiple_pools_independent() {
        let mut detector = JitDetector::new(1);

        detector.process_tx(0, &[v3_mint_log(pool_a(), -100, 100, 500_000)], None);
        detector.process_tx(1, &[v3_mint_log(pool_b(), -200, 200, 1_000_000)], None);
        detector.process_tx(2, &[v3_swap_log(pool_a())], None);

        let opps = detector.detect(100);
        // Only pool_a has Mint+Swap (partial JIT), pool_b hasn't been swapped
        assert_eq!(opps.len(), 1);
        assert_eq!(opps[0].pool_a, pool_a());
    }

    #[test]
    fn test_no_duplicate_emission() {
        let mut detector = JitDetector::new(1);

        detector.process_tx(0, &[v3_mint_log(pool_a(), -100, 100, 500_000)], None);
        detector.process_tx(1, &[v3_swap_log(pool_a())], None);

        // First detect
        let opps = detector.detect(100);
        assert_eq!(opps.len(), 1);

        // Second detect (same state, no new events)
        let opps2 = detector.detect(100);
        assert!(opps2.is_empty(), "Should not re-emit same opportunity");
    }

    #[test]
    fn test_mint_only_no_detection() {
        let mut detector = JitDetector::new(1);
        detector.process_tx(0, &[v3_mint_log(pool_a(), -100, 100, 500_000)], None);
        // No swap, no burn
        let opps = detector.detect(100);
        assert!(opps.is_empty());
    }

    #[test]
    fn test_swap_burn_without_mint_no_detection() {
        let mut detector = JitDetector::new(1);
        // Burn without prior mint (should be ignored)
        detector.process_tx(0, &[v3_burn_log(pool_a(), -100, 100, 500_000)], None);
        detector.process_tx(1, &[v3_swap_log(pool_a())], None);
        let opps = detector.detect(100);
        assert!(opps.is_empty());
    }
}
