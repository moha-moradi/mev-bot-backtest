use alloy::primitives::{Address, B256, Bytes, U256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockData {
    pub number: u64,
    pub hash: B256,
    pub timestamp: u64,
    pub base_fee_per_gas: Option<u128>,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub coinbase: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxData {
    pub hash: B256,
    pub index: u64,
    pub from: Address,
    pub to: Option<Address>,
    pub input: Bytes,
    pub value: U256,
    pub gas_limit: u64,
    pub max_fee_per_gas: u128,
    pub max_priority_fee_per_gas: Option<u128>,
    pub nonce: u64,
    pub access_list: Vec<AccessListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessListItem {
    pub address: Address,
    pub slots: Vec<B256>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptData {
    pub tx_hash: B256,
    pub tx_index: u64,
    pub status: bool,
    pub gas_used: u64,
    pub cumulative_gas_used: u64,
    pub logs: Vec<LogData>,
    pub contract_address: Option<Address>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogData {
    pub address: Address,
    pub topics: Vec<B256>,
    pub data: Bytes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountData {
    pub nonce: u64,
    pub balance: U256,
    pub code_hash: B256,
}

#[derive(Debug, Clone)]
pub struct ExecutedTx {
    pub tx_hash: B256,
    pub index: u64,
    pub status: bool,
    pub gas_used: u64,
    pub gas_effective: u128,
    pub logs: Vec<ExecutedLog>,
    pub output: Bytes,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExecutedLog {
    pub address: Address,
    pub topics: Vec<B256>,
    pub data: Bytes,
}
