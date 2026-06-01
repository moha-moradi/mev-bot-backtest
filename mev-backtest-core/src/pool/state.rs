use std::collections::HashMap;

use alloy::primitives::{b256, Address, Bytes, B256, U256};
use serde::{Deserialize, Serialize};

use crate::data::ExecutedLog;
use crate::rpc::RpcClient;

/// Static pool information loaded from the registry JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub address: Address,
    #[serde(rename = "type")]
    pub pool_type: String,
    pub token0: Address,
    pub token1: Address,
    pub fee: u32,
    pub name: Option<String>,
}

/// Runtime state for a Uniswap V2 constant-product pool.
#[derive(Debug, Clone)]
pub struct UniswapV2PoolState {
    pub info: PoolInfo,
    pub reserve0: u128,
    pub reserve1: u128,
}

/// Runtime state for any tracked pool.
#[derive(Debug, Clone)]
pub enum PoolState {
    UniswapV2(UniswapV2PoolState),
}

impl PoolState {
    pub fn address(&self) -> Address {
        match self {
            PoolState::UniswapV2(s) => s.info.address,
        }
    }

    pub fn info(&self) -> &PoolInfo {
        match self {
            PoolState::UniswapV2(s) => &s.info,
        }
    }
}

/// Event signature for Uniswap V2 Swap event
const SWAP_TOPIC: B256 = b256!("d78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822");
/// Event signature for Uniswap V2 Sync event
const SYNC_TOPIC: B256 = b256!("1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1");
/// getReserves() selector
const GET_RESERVES_SELECTOR: [u8; 4] = [0x09, 0x02, 0xf1, 0xac];

/// Manages runtime pool state: initializes from RPC, updates from Swap/Sync events.
#[derive(Debug, Clone)]
pub struct PoolManager {
    pools: HashMap<Address, PoolState>,
    /// token address -> list of pool addresses that trade this token
    token_index: HashMap<Address, Vec<Address>>,
}

impl PoolManager {
    pub fn new() -> Self {
        PoolManager {
            pools: HashMap::new(),
            token_index: HashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        PoolManager {
            pools: HashMap::with_capacity(capacity),
            token_index: HashMap::with_capacity(capacity),
        }
    }

    pub fn add_pool(&mut self, state: PoolState) {
        let addr = state.address();
        let info = state.info().clone();
        self.pools.insert(addr, state);
        self.token_index
            .entry(info.token0)
            .or_default()
            .push(addr);
        self.token_index
            .entry(info.token1)
            .or_default()
            .push(addr);
    }

    pub fn get(&self, address: &Address) -> Option<&PoolState> {
        self.pools.get(address)
    }

    pub fn get_mut(&mut self, address: &Address) -> Option<&mut PoolState> {
        self.pools.get_mut(address)
    }

    pub fn all_pools(&self) -> impl Iterator<Item = &PoolState> {
        self.pools.values()
    }

    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }

    pub fn pool_addresses(&self) -> Vec<Address> {
        self.pools.keys().copied().collect()
    }

    /// Returns pairs of pool addresses that share at least one common token.
    /// Each pair is returned once (pool_a < pool_b by address), with the shared token.
    pub fn arbitrage_pairs(&self) -> Vec<(Address, Address, Address)> {
        let mut pairs = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for (_token, pool_addrs) in &self.token_index {
            for i in 0..pool_addrs.len() {
                for j in (i + 1)..pool_addrs.len() {
                    let a = pool_addrs[i];
                    let b = pool_addrs[j];
                    let key = if a < b { (a, b) } else { (b, a) };
                    if seen.insert(key) {
                        pairs.push((key.0, key.1, *_token));
                    }
                }
            }
        }

        pairs
    }

    /// Update a V2 pool's reserves using amounts from a Swap event.
    pub fn apply_v2_swap(
        &mut self,
        address: &Address,
        amount0_in: u128,
        amount1_in: u128,
        amount0_out: u128,
        amount1_out: u128,
    ) {
        if let Some(PoolState::UniswapV2(state)) = self.pools.get_mut(address) {
            state.reserve0 = state.reserve0.wrapping_add(amount0_in).wrapping_sub(amount0_out);
            state.reserve1 = state.reserve1.wrapping_add(amount1_in).wrapping_sub(amount1_out);
        }
    }

    /// Update a V2 pool's reserves from a Sync event (authoritative override).
    pub fn apply_v2_sync(&mut self, address: &Address, reserve0: u128, reserve1: u128) {
        if let Some(PoolState::UniswapV2(state)) = self.pools.get_mut(address) {
            state.reserve0 = reserve0;
            state.reserve1 = reserve1;
        }
    }

    /// Count pools that have non-zero reserves (i.e., initialized).
    pub fn initialized_count(&self) -> usize {
        self.pools
            .values()
            .filter(|p| match p {
                PoolState::UniswapV2(s) => s.reserve0 > 0 && s.reserve1 > 0,
            })
            .count()
    }

    /// Initialize pool reserves from on-chain `getReserves()` calls at a historical block.
    pub async fn init_from_rpc(&mut self, rpc: &RpcClient, block_num: u64) {
        let pool_addrs: Vec<Address> = self.pools.keys().copied().collect();
        for addr in &pool_addrs {
            match Self::fetch_v2_reserves(rpc, *addr, block_num).await {
                Some((r0, r1)) => {
                    if let Some(PoolState::UniswapV2(state)) = self.pools.get_mut(addr) {
                        state.reserve0 = r0;
                        state.reserve1 = r1;
                    }
                }
                None => {
                    tracing::warn!("Failed to fetch reserves for pool {}", addr);
                }
            }
        }
    }

    async fn fetch_v2_reserves(rpc: &RpcClient, pool: Address, block: u64) -> Option<(u128, u128)> {
        let data = Bytes::copy_from_slice(&GET_RESERVES_SELECTOR);
        let result = rpc.call(pool, data, block).await.ok()?;
        if result.len() < 64 {
            return None;
        }
        // ABI decode: 3 × uint256 (reserve0, reserve1, blockTimestampLast), but reserve0/1 are uint112
        // Alloy returns 0-padded bytes; read the last 32 bytes for each value
        let mut buf = [0u8; 32];
        buf.copy_from_slice(&result[..32]);
        let r0 = U256::from_be_bytes(buf).as_limbs()[0] as u128;
        buf.copy_from_slice(&result[32..64]);
        let r1 = U256::from_be_bytes(buf).as_limbs()[0] as u128;
        Some((r0, r1))
    }

    /// Process a list of executed logs from a single transaction, updating pool state
    /// for any Swap or Sync events emitted by tracked pools.
    pub fn update_from_logs(&mut self, logs: &[ExecutedLog]) {
        for log in logs {
            if log.topics.is_empty() {
                continue;
            }
            let topic0 = log.topics[0];
            if topic0 == SWAP_TOPIC {
                self.process_swap_log(log);
            } else if topic0 == SYNC_TOPIC {
                self.process_sync_log(log);
            }
        }
    }

    fn process_swap_log(&mut self, log: &ExecutedLog) {
        if !self.pools.contains_key(&log.address) {
            return;
        }
        if log.data.len() < 128 {
            return;
        }
        let amt0_in = u128_from_be_bytes(&log.data[..32]);
        let amt1_in = u128_from_be_bytes(&log.data[32..64]);
        let amt0_out = u128_from_be_bytes(&log.data[64..96]);
        let amt1_out = u128_from_be_bytes(&log.data[96..128]);
        self.apply_v2_swap(&log.address, amt0_in, amt1_in, amt0_out, amt1_out);
    }

    fn process_sync_log(&mut self, log: &ExecutedLog) {
        if !self.pools.contains_key(&log.address) {
            return;
        }
        if log.data.len() < 64 {
            return;
        }
        let r0 = u128_from_be_bytes(&log.data[..32]);
        let r1 = u128_from_be_bytes(&log.data[32..64]);
        self.apply_v2_sync(&log.address, r0, r1);
    }
}

impl Default for PoolManager {
    fn default() -> Self {
        Self::new()
    }
}

fn u128_from_be_bytes(bytes: &[u8]) -> u128 {
    let mut buf = [0u8; 16];
    let start = bytes.len().saturating_sub(16);
    buf.copy_from_slice(&bytes[start..start + 16]);
    u128::from_be_bytes(buf)
}
