use std::path::Path;

use alloy::primitives::{Address, Bytes, U256};
use serde::{Deserialize, Serialize};

use crate::data::{AccountData, BlockData, ReceiptData, TxData};
use crate::pool::state::PoolInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunManifest {
    pub run_id: String,
    pub chain: String,
    pub start_block: u64,
    pub end_block: u64,
    pub resolved_at: u64,
    pub range_mode: String,
    pub strategies: Vec<String>,
    pub flash_loan_provider: String,
}

#[derive(Clone)]
pub struct CacheStore {
    db: sled::Db,
    chain_id: u64,
}

impl CacheStore {
    pub fn open(path: impl AsRef<Path>, chain_id: u64) -> anyhow::Result<Self> {
        let db = sled::open(path)?;
        Ok(CacheStore { db, chain_id })
    }

    fn key(&self, prefix: &str, parts: &[&dyn std::fmt::Display]) -> String {
        let mut k = format!("{}:{}", prefix, self.chain_id);
        for p in parts {
            k.push(':');
            k.push_str(&p.to_string());
        }
        k
    }

    fn encode<T: Serialize + ?Sized>(val: &T) -> anyhow::Result<Vec<u8>> {
        Ok(bincode::serialize(val)?)
    }

    fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> anyhow::Result<T> {
        Ok(bincode::deserialize(bytes)?)
    }

    // ---- Block ----
    pub fn put_block(&self, block_num: u64, block: &BlockData) -> anyhow::Result<()> {
        let key = self.key("block", &[&block_num]);
        self.db.insert(key, Self::encode(block)?)?;
        Ok(())
    }

    pub fn get_block(&self, block_num: u64) -> anyhow::Result<Option<BlockData>> {
        let key = self.key("block", &[&block_num]);
        match self.db.get(&key)? {
            Some(bytes) => Ok(Some(Self::decode(&bytes)?)),
            None => Ok(None),
        }
    }

    // ---- Txs ----
    pub fn put_txs(&self, block_num: u64, txs: &[TxData]) -> anyhow::Result<()> {
        let key = self.key("txs", &[&block_num]);
        self.db.insert(key, Self::encode(txs)?)?;
        Ok(())
    }

    pub fn get_txs(&self, block_num: u64) -> anyhow::Result<Option<Vec<TxData>>> {
        let key = self.key("txs", &[&block_num]);
        match self.db.get(&key)? {
            Some(bytes) => Ok(Some(Self::decode(&bytes)?)),
            None => Ok(None),
        }
    }

    // ---- Receipts ----
    pub fn put_receipts(&self, block_num: u64, receipts: &[ReceiptData]) -> anyhow::Result<()> {
        let key = self.key("receipts", &[&block_num]);
        self.db.insert(key, Self::encode(receipts)?)?;
        Ok(())
    }

    pub fn get_receipts(&self, block_num: u64) -> anyhow::Result<Option<Vec<ReceiptData>>> {
        let key = self.key("receipts", &[&block_num]);
        match self.db.get(&key)? {
            Some(bytes) => Ok(Some(Self::decode(&bytes)?)),
            None => Ok(None),
        }
    }

    // ---- Check integrity ----
    pub fn has_block(&self, block_num: u64) -> anyhow::Result<bool> {
        Ok(self.get_block(block_num)?.is_some()
            && self.get_txs(block_num)?.is_some()
            && self.get_receipts(block_num)?.is_some())
    }

    pub fn check_integrity(&self, start: u64, end: u64) -> anyhow::Result<Vec<u64>> {
        let mut missing = Vec::new();
        for n in start..=end {
            if !self.has_block(n)? {
                missing.push(n);
            }
        }
        Ok(missing)
    }

    // ---- Account / Slot / Code (lazy fetch target for revm) ----
    pub fn put_account(
        &self,
        block_num: u64,
        address: Address,
        account: &AccountData,
    ) -> anyhow::Result<()> {
        let key = self.key("account", &[&block_num, &address]);
        self.db.insert(key, Self::encode(account)?)?;
        Ok(())
    }

    pub fn get_account(
        &self,
        block_num: u64,
        address: Address,
    ) -> anyhow::Result<Option<AccountData>> {
        let key = self.key("account", &[&block_num, &address]);
        match self.db.get(&key)? {
            Some(bytes) => Ok(Some(Self::decode(&bytes)?)),
            None => Ok(None),
        }
    }

    pub fn put_slot(
        &self,
        block_num: u64,
        address: Address,
        slot: U256,
        value: U256,
    ) -> anyhow::Result<()> {
        let key = self.key("slot", &[&block_num, &address, &slot]);
        self.db.insert(key, Self::encode(&value)?)?;
        Ok(())
    }

    pub fn get_slot(
        &self,
        block_num: u64,
        address: Address,
        slot: U256,
    ) -> anyhow::Result<Option<U256>> {
        let key = self.key("slot", &[&block_num, &address, &slot]);
        match self.db.get(&key)? {
            Some(bytes) => Ok(Some(Self::decode(&bytes)?)),
            None => Ok(None),
        }
    }

    pub fn put_code(&self, address: Address, code: &Bytes) -> anyhow::Result<()> {
        let key = self.key("code", &[&address]);
        self.db.insert(key, Self::encode(code)?)?;
        Ok(())
    }

    pub fn get_code(&self, address: Address) -> anyhow::Result<Option<Bytes>> {
        let key = self.key("code", &[&address]);
        match self.db.get(&key)? {
            Some(bytes) => Ok(Some(Self::decode(&bytes)?)),
            None => Ok(None),
        }
    }

    // ---- RunManifest ----
    pub fn put_manifest(&self, manifest: &RunManifest) -> anyhow::Result<()> {
        let key = format!("manifest:{}", manifest.run_id);
        self.db.insert(key, Self::encode(manifest)?)?;
        Ok(())
    }

    pub fn get_manifest(&self, run_id: &str) -> anyhow::Result<Option<RunManifest>> {
        let key = format!("manifest:{}", run_id);
        match self.db.get(&key)? {
            Some(bytes) => Ok(Some(Self::decode(&bytes)?)),
            None => Ok(None),
        }
    }

    /// List all stored run manifests with their run IDs.
    pub fn list_manifests(&self) -> anyhow::Result<Vec<(String, RunManifest)>> {
        let prefix = "manifest:";
        let mut results = Vec::new();
        for entry in self.db.scan_prefix(prefix.as_bytes()) {
            let (key, value) = entry?;
            let key_str = String::from_utf8_lossy(&key).to_string();
            if let Some(run_id) = key_str.strip_prefix(prefix) {
                if let Ok(manifest) = Self::decode::<RunManifest>(&value) {
                    results.push((run_id.to_string(), manifest));
                }
            }
        }
        results.sort_by(|a, b| b.1.resolved_at.cmp(&a.1.resolved_at));
        Ok(results)
    }

    // ---- Pool Discovery ----
    pub fn put_discovered_pool(&self, pool: &PoolInfo) -> anyhow::Result<()> {
        let key = format!("discovery:{}:pool:{}", self.chain_id, pool.address);
        self.db.insert(key, Self::encode(pool)?)?;
        Ok(())
    }

    pub fn get_discovered_pool(&self, address: &Address) -> anyhow::Result<Option<PoolInfo>> {
        let key = format!("discovery:{}:pool:{}", self.chain_id, address);
        match self.db.get(&key)? {
            Some(bytes) => Ok(Some(Self::decode(&bytes)?)),
            None => Ok(None),
        }
    }

    pub fn list_discovered_pools(&self) -> anyhow::Result<Vec<PoolInfo>> {
        let prefix = format!("discovery:{}:pool:", self.chain_id);
        let mut pools = Vec::new();
        for entry in self.db.scan_prefix(prefix.as_bytes()) {
            let (_, value) = entry?;
            if let Ok(pool) = Self::decode::<PoolInfo>(&value) {
                pools.push(pool);
            }
        }
        Ok(pools)
    }

    pub fn put_discovery_cursor(&self, factory: &Address, block: u64) -> anyhow::Result<()> {
        let key = format!("discovery:{}:cursor:{}", self.chain_id, factory);
        self.db.insert(key, Self::encode(&block)?)?;
        Ok(())
    }

    pub fn get_discovery_cursor(&self, factory: &Address) -> anyhow::Result<Option<u64>> {
        let key = format!("discovery:{}:cursor:{}", self.chain_id, factory);
        match self.db.get(&key)? {
            Some(bytes) => Ok(Some(Self::decode(&bytes)?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::{address, B256, U256};
    use crate::data::{AccountData, BlockData, ReceiptData, TxData};
    use crate::pool::dex_type::DexType;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static CACHE_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_cache() -> CacheStore {
        let id = CACHE_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("mev_ut_cache_{}", id));
        let _ = std::fs::remove_dir_all(&dir);
        CacheStore::open(&dir, 137).unwrap()
    }

    #[test]
    fn test_put_get_block() {
        let cache = temp_cache();
        let block = BlockData {
            number: 42,
            hash: B256::from([1u8; 32]),
            timestamp: 1000,
            base_fee_per_gas: Some(50_000_000_000),
            gas_limit: 30_000_000,
            gas_used: 15_000_000,
            coinbase: address!("dead000000000000000000000000000000000000"),
        };
        cache.put_block(42, &block).unwrap();
        let fetched = cache.get_block(42).unwrap().unwrap();
        assert_eq!(fetched.number, 42);
        assert_eq!(fetched.hash, block.hash);
        assert_eq!(fetched.timestamp, 1000);
    }

    #[test]
    fn test_put_get_txs() {
        let cache = temp_cache();
        let txs = vec![TxData {
            hash: B256::from([2u8; 32]),
            index: 0,
            from: address!("aa00000000000000000000000000000000000000"),
            to: Some(address!("bb00000000000000000000000000000000000000")),
            input: vec![0x12, 0x34].into(),
            value: U256::from(1000u64),
            gas_limit: 100_000,
            max_fee_per_gas: 100_000_000_000,
            max_priority_fee_per_gas: Some(1_000_000_000),
            nonce: 5,
            access_list: vec![],
        }];
        cache.put_txs(42, &txs).unwrap();
        let fetched = cache.get_txs(42).unwrap().unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].nonce, 5);
    }

    #[test]
    fn test_put_get_receipts() {
        let cache = temp_cache();
        let receipts = vec![ReceiptData {
            tx_hash: B256::from([3u8; 32]),
            tx_index: 0,
            status: true,
            gas_used: 50_000,
            cumulative_gas_used: 50_000,
            logs: vec![],
            contract_address: None,
        }];
        cache.put_receipts(42, &receipts).unwrap();
        let fetched = cache.get_receipts(42).unwrap().unwrap();
        assert_eq!(fetched.len(), 1);
        assert!(fetched[0].status);
        assert_eq!(fetched[0].gas_used, 50_000);
    }

    #[test]
    fn test_has_block_and_check_integrity() {
        let cache = temp_cache();
        let block = BlockData {
            number: 1, hash: B256::ZERO, timestamp: 0,
            base_fee_per_gas: None, gas_limit: 0, gas_used: 0, coinbase: Address::ZERO,
        };
        cache.put_block(1, &block).unwrap();
        assert!(!cache.has_block(1).unwrap());
        cache.put_txs(1, &[]).unwrap();
        cache.put_receipts(1, &[]).unwrap();
        assert!(cache.has_block(1).unwrap());
        let missing = cache.check_integrity(1, 3).unwrap();
        assert_eq!(missing, vec![2, 3]);
    }

    #[test]
    fn test_get_nonexistent() {
        let cache = temp_cache();
        assert!(cache.get_block(999).unwrap().is_none());
        assert!(cache.get_txs(999).unwrap().is_none());
        assert!(cache.get_receipts(999).unwrap().is_none());
    }

    #[test]
    fn test_account_roundtrip() {
        let cache = temp_cache();
        let addr = address!("abcdef0000000000000000000000000000000001");
        let acc = AccountData {
            nonce: 10,
            balance: U256::from(1_000_000_000u64),
            code_hash: B256::from([4u8; 32]),
        };
        cache.put_account(42, addr, &acc).unwrap();
        let fetched = cache.get_account(42, addr).unwrap().unwrap();
        assert_eq!(fetched.nonce, 10);
        assert_eq!(fetched.balance, U256::from(1_000_000_000u64));
    }

    #[test]
    fn test_slot_roundtrip() {
        let cache = temp_cache();
        let addr = address!("abcdef0000000000000000000000000000000002");
        cache.put_slot(42, addr, U256::from(7u64), U256::from(42u64)).unwrap();
        let fetched = cache.get_slot(42, addr, U256::from(7u64)).unwrap().unwrap();
        assert_eq!(fetched, U256::from(42u64));
    }

    #[test]
    fn test_code_roundtrip() {
        let cache = temp_cache();
        let addr = address!("abcdef0000000000000000000000000000000003");
        let code = Bytes::from(vec![0x60, 0x00, 0x52]);
        cache.put_code(addr, &code).unwrap();
        let fetched = cache.get_code(addr).unwrap().unwrap();
        assert_eq!(fetched, code);
    }

    #[test]
    fn test_manifest_roundtrip() {
        let cache = temp_cache();
        let manifest = RunManifest {
            run_id: "test-run-1".into(),
            chain: "polygon".into(),
            start_block: 1,
            end_block: 100,
            resolved_at: 1000,
            range_mode: "blocks".into(),
            strategies: vec!["two_hop_arb".into()],
            flash_loan_provider: "auto".into(),
        };
        cache.put_manifest(&manifest).unwrap();
        let fetched = cache.get_manifest("test-run-1").unwrap().unwrap();
        assert_eq!(fetched.run_id, "test-run-1");
        assert_eq!(fetched.start_block, 1);
        assert_eq!(fetched.end_block, 100);
    }

    #[test]
    fn test_discovered_pool_roundtrip() {
        let cache = temp_cache();
        let pool = PoolInfo {
            address: address!("cafe000000000000000000000000000000000001"),
            token0: address!("aaaa0000000000000000000000000000000000aa"),
            token1: address!("bbbb0000000000000000000000000000000000bb"),
            fee: 3000,
            name: None,
            dex_type: DexType::UniswapV2,
            tick_spacing: None,
        };
        cache.put_discovered_pool(&pool).unwrap();
        let fetched = cache.get_discovered_pool(&pool.address).unwrap().unwrap();
        assert_eq!(fetched.address, pool.address);
        assert_eq!(fetched.dex_type, DexType::UniswapV2);
        let all = cache.list_discovered_pools().unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn test_discovery_cursor_roundtrip() {
        let cache = temp_cache();
        let factory = Address::from_slice(&[0xfa; 20]);
        assert!(cache.get_discovery_cursor(&factory).unwrap().is_none());
        cache.put_discovery_cursor(&factory, 42_000_000).unwrap();
        let cursor = cache.get_discovery_cursor(&factory).unwrap().unwrap();
        assert_eq!(cursor, 42_000_000);
    }

    #[test]
    fn test_discovery_namespaced_by_chain() {
        let dir_a = std::env::temp_dir().join("disc_test_a");
        let dir_b = std::env::temp_dir().join("disc_test_b");
        let _ = std::fs::remove_dir_all(&dir_a);
        let _ = std::fs::remove_dir_all(&dir_b);
        let cache_a = CacheStore::open(&dir_a, 137).unwrap();
        let cache_b = CacheStore::open(&dir_b, 31337).unwrap();
        let pool = PoolInfo {
            address: address!("cafe000000000000000000000000000000000002"),
            token0: Address::ZERO,
            token1: Address::ZERO,
            fee: 0,
            name: None,
            dex_type: DexType::UniswapV2,
            tick_spacing: None,
        };
        cache_a.put_discovered_pool(&pool).unwrap();
        assert_eq!(cache_b.list_discovered_pools().unwrap().len(), 0);
        assert_eq!(cache_a.list_discovered_pools().unwrap().len(), 1);
    }
}
