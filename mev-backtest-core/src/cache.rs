use std::path::Path;

use alloy::primitives::{Address, Bytes, U256};
use serde::{Deserialize, Serialize};

use crate::data::{AccountData, BlockData, ReceiptData, TxData};

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
}
