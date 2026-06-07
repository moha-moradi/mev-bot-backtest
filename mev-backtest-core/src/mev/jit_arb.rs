use std::collections::HashMap;
use alloy::primitives::{Address, U256};
use crate::data::ExecutedLog;
use crate::pool::decoders::{decode_v3_mint_burn, V3_SWAP_TOPIC, V3_MINT_TOPIC, V3_BURN_TOPIC};
use crate::pool::state::PoolManager;
use crate::mev::opportunity::MevOpportunity;
use crate::types::Strategy;

#[derive(Debug, Clone)]
struct SwapEvent {
    tx_index: usize,
    pool: Address,
    sender: Address,
}

#[derive(Debug, Clone)]
struct JitArbMint {
    mint_tx_index: usize,
    tick_lower: i32,
    tick_upper: i32,
    amount: u128,
    sender: Address,
    swapped: bool,
    burned: bool,
}

pub struct JitArbDetector {
    active_mints: HashMap<Address, Vec<JitArbMint>>,
    swap_events: Vec<SwapEvent>,
    emitted: Vec<(Address, usize, Address)>,
    block_number: u64,
}

impl JitArbDetector {
    pub fn new(block_number: u64) -> Self {
        JitArbDetector {
            active_mints: HashMap::new(),
            swap_events: Vec::new(),
            emitted: Vec::new(),
            block_number,
        }
    }

    pub fn process_tx(&mut self, tx_index: usize, logs: &[ExecutedLog], sender: Option<Address>) {
        let sender = match sender {
            Some(s) => s,
            None => return,
        };

        for log in logs {
            if log.topics.is_empty() {
                continue;
            }
            let t0 = log.topics[0];

            // Mint event
            if t0 == *V3_MINT_TOPIC {
                if let Some(decoded) = decode_v3_mint_burn(log) {
                    if decoded.amount > 0 {
                        self.active_mints
                            .entry(log.address)
                            .or_default()
                            .push(JitArbMint {
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
            }

            // Burn event
            if t0 == V3_BURN_TOPIC {
                if let Some(decoded) = decode_v3_mint_burn(log) {
                    if let Some(mints) = self.active_mints.get_mut(&log.address) {
                        for mint in mints.iter_mut() {
                            if mint.burned { continue; }
                            if mint.sender == sender
                                && mint.tick_lower == decoded.tick_lower
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

            // Swap event
            if t0 == V3_SWAP_TOPIC {
                self.swap_events.push(SwapEvent {
                    tx_index,
                    pool: log.address,
                    sender,
                });
                // Mark any matching active mints on this pool as swapped
                if let Some(mints) = self.active_mints.get_mut(&log.address) {
                    for mint in mints.iter_mut() {
                        if mint.sender == sender && mint.mint_tx_index <= tx_index {
                            mint.swapped = true;
                        }
                    }
                }
            }
        }
    }

    pub fn detect(&mut self, timestamp: u64, pm: &PoolManager) -> Vec<MevOpportunity> {
        let mut opportunities = Vec::new();
        let pool_addrs: Vec<Address> = self.active_mints.keys().copied().collect();

        for &pool_p in &pool_addrs {
            let Some(mints) = self.active_mints.get(&pool_p) else { continue };
            for mint in mints {
                let dedup_key = (pool_p, mint.mint_tx_index, mint.sender);
                if self.emitted.contains(&dedup_key) || !mint.swapped {
                    continue;
                }

                // Find swaps on pool_p by same sender
                let swaps_on_p: Vec<&SwapEvent> = self.swap_events.iter()
                    .filter(|s| s.pool == pool_p && s.sender == mint.sender && s.tx_index >= mint.mint_tx_index)
                    .collect();
                if swaps_on_p.is_empty() {
                    continue;
                }

                // Find swaps on a different pool Q sharing a token
                for swap_p in &swaps_on_p {
                    for swap_q in &self.swap_events {
                        if swap_q.pool == pool_p || swap_q.sender != mint.sender {
                            continue;
                        }
                        // Check proximity: within 1 tx index
                        let p_idx = swap_p.tx_index;
                        let q_idx = swap_q.tx_index;
                        let max_idx = p_idx.max(q_idx);
                        let min_idx = p_idx.min(q_idx);
                        if max_idx - min_idx > 1 {
                            continue;
                        }
                        // Check token sharing
                        if pools_share_token(pm, pool_p, swap_q.pool) {
                            self.emitted.push(dedup_key);
                            opportunities.push(Self::build_opp(
                                self.block_number, pool_p, swap_q.pool, mint, timestamp,
                            ));
                            break;
                        }
                    }
                    if !opportunities.is_empty() { break; }
                }
            }
        }

        opportunities
    }

    fn build_opp(
        block_number: u64,
        jit_pool: Address,
        arb_pool: Address,
        mint: &JitArbMint,
        timestamp: u64,
    ) -> MevOpportunity {
        MevOpportunity {
            block_number,
            tx_index: mint.mint_tx_index,
            strategy: Strategy::JitArb,
            pool_a: jit_pool,
            pool_b: arb_pool,
            token_in: Address::ZERO,
            token_out: Address::ZERO,
            input_amount: U256::from(mint.amount),
            expected_profit: U256::ZERO,
            gas_cost_wei: 0,
            timestamp,
            path: Some(vec![jit_pool, arb_pool]),
            tick_lower: Some(mint.tick_lower),
            tick_upper: Some(mint.tick_upper),
            liquidity_amount: Some(mint.amount),
            victim_tx_index: None,
            backrun_tx_index: None,
        }
    }
}

fn pools_share_token(pm: &PoolManager, pool_a: Address, pool_b: Address) -> bool {
    let Some(info_a) = pm.get(&pool_a).map(|p| p.info()) else { return false };
    let Some(info_b) = pm.get(&pool_b).map(|p| p.info()) else { return false };
    info_a.token0 == info_b.token0
        || info_a.token0 == info_b.token1
        || info_a.token1 == info_b.token0
        || info_a.token1 == info_b.token1
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::{address, Bytes, B256};
    use crate::pool::state::{PoolManager, UniswapV2PoolState, PoolInfo, PoolState};

    fn pool_p() -> Address { address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa") }
    fn pool_q() -> Address { address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb") }
    fn wmatic() -> Address { address!("0d500b1d8e8ef31e21c99d1db9a6444d3adf1270") }
    fn usdc() -> Address { address!("2791bca1f2de4661ed88a30c99a7a9449aa84174") }
    fn sender() -> Address { address!("1111111111111111111111111111111111111111") }

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
        ExecutedLog { address: pool, topics: vec![*V3_MINT_TOPIC, B256::ZERO, B256::ZERO], data: data.into() }
    }

    fn v3_swap_log(pool: Address) -> ExecutedLog {
        ExecutedLog { address: pool, topics: vec![V3_SWAP_TOPIC, B256::ZERO, B256::ZERO], data: Bytes::from_static(&[0u8; 160]) }
    }

    fn make_pm() -> PoolManager {
        let mut pm = PoolManager::new();
        pm.add_pool(PoolState::UniswapV2(UniswapV2PoolState {
            info: PoolInfo {
                address: pool_p(), token0: wmatic(), token1: usdc(), fee: 30, name: None,
                dex_type: crate::pool::dex_type::DexType::UniswapV2, tick_spacing: None,
            },
            reserve0: 1_000_000, reserve1: 1_000_000,
        }));
        pm.add_pool(PoolState::UniswapV2(UniswapV2PoolState {
            info: PoolInfo {
                address: pool_q(), token0: usdc(),
                token1: address!("c2132d05d31c914a87c6611c10748aeb04b58e8f"),
                fee: 30, name: None,
                dex_type: crate::pool::dex_type::DexType::UniswapV2, tick_spacing: None,
            },
            reserve0: 1_000_000, reserve1: 1_000_000,
        }));
        pm
    }

    #[test]
    fn test_empty_detector_returns_nothing() {
        let mut detector = JitArbDetector::new(1);
        let pm = PoolManager::new();
        let opps = detector.detect(100, &pm);
        assert!(opps.is_empty());
    }

    #[test]
    fn test_mint_and_arb_same_tx() {
        let mut detector = JitArbDetector::new(1);
        let pm = make_pm();
        detector.process_tx(0, &[
            v3_mint_log(pool_p(), -100, 100, 500_000),
            v3_swap_log(pool_p()),
            v3_swap_log(pool_q()),
        ], Some(sender()));
        let opps = detector.detect(100, &pm);
        assert_eq!(opps.len(), 1, "Same-tx Mint+arb should be detected");
        assert_eq!(opps[0].strategy, Strategy::JitArb);
        assert_eq!(opps[0].pool_a, pool_p());
        assert_eq!(opps[0].pool_b, pool_q());
        assert_eq!(opps[0].liquidity_amount, Some(500_000));
    }

    #[test]
    fn test_mint_then_arb_cross_tx() {
        let mut detector = JitArbDetector::new(1);
        let pm = make_pm();
        detector.process_tx(0, &[v3_mint_log(pool_p(), -100, 100, 500_000)], Some(sender()));
        assert!(detector.detect(100, &pm).is_empty(), "Mint alone should not trigger JitArb");
        detector.process_tx(1, &[v3_swap_log(pool_p()), v3_swap_log(pool_q())], Some(sender()));
        let opps = detector.detect(100, &pm);
        assert_eq!(opps.len(), 1, "Cross-tx Mint+arb should be detected");
    }

    #[test]
    fn test_different_sender_no_detection() {
        let mut detector = JitArbDetector::new(1);
        let pm = make_pm();
        let other = address!("2222222222222222222222222222222222222222");
        detector.process_tx(0, &[v3_mint_log(pool_p(), -100, 100, 500_000)], Some(sender()));
        detector.process_tx(1, &[v3_swap_log(pool_p()), v3_swap_log(pool_q())], Some(other));
        let opps = detector.detect(100, &pm);
        assert!(opps.is_empty(), "Different sender should not trigger JitArb");
    }

    #[test]
    fn test_no_token_share_no_detection() {
        let mut detector = JitArbDetector::new(1);
        let pm = {
            let mut pm = make_pm();
            let pool_r = address!("cccccccccccccccccccccccccccccccccccccccc");
            pm.add_pool(PoolState::UniswapV2(UniswapV2PoolState {
                info: PoolInfo {
                    address: pool_r,
                    token0: address!("8f3cf7ad23cd3cadbd9735aff958023239c6a063"),
                    token1: address!("53e0bca35ec356bd5dddfebbd1fc0fd03fabad39"),
                    fee: 30, name: None,
                    dex_type: crate::pool::dex_type::DexType::UniswapV2, tick_spacing: None,
                },
                reserve0: 1_000_000, reserve1: 1_000_000,
            }));
            pm
        };
        detector.process_tx(0, &[
            v3_mint_log(pool_p(), -100, 100, 500_000),
            v3_swap_log(pool_p()),
            v3_swap_log(address!("cccccccccccccccccccccccccccccccccccccccc")),
        ], Some(sender()));
        let opps = detector.detect(100, &pm);
        assert!(opps.is_empty(), "No token sharing should not trigger JitArb");
    }

    #[test]
    fn test_no_duplicate_emission() {
        let mut detector = JitArbDetector::new(1);
        let pm = make_pm();
        detector.process_tx(0, &[
            v3_mint_log(pool_p(), -100, 100, 500_000),
            v3_swap_log(pool_p()),
            v3_swap_log(pool_q()),
        ], Some(sender()));
        assert_eq!(detector.detect(100, &pm).len(), 1);
        assert!(detector.detect(100, &pm).is_empty(), "Should not re-emit");
    }
}
