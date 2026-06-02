use alloy::primitives::{b256, keccak256, Address, B256, Bytes};

use crate::rpc::RpcClient;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactoryStatus {
    Verified { name: String, code_hash: B256 },
    Unknown(String),
    NoCode,
}

/// Verify a factory address has non-empty bytecode and return its code hash.
pub async fn verify_factory(
    rpc: &RpcClient,
    address: Address,
    block: u64,
) -> anyhow::Result<FactoryStatus> {
    let code: Bytes = rpc.get_code(address, block).await?;

    if code.is_empty() {
        return Ok(FactoryStatus::NoCode);
    }

    let code_hash = keccak256(&code);
    let name = match identify_factory(&code_hash) {
        Some(n) => n,
        None => return Ok(FactoryStatus::Unknown(format!("{:#x}", code_hash))),
    };

    Ok(FactoryStatus::Verified { name: name.to_string(), code_hash })
}

/// Try to identify a factory by its code hash.
pub fn identify_factory(code_hash: &B256) -> Option<&'static str> {
    // Uniswap V2 (also used by QuickSwap, PancakeSwap on some chains)
    if *code_hash == b256!("96e8ac4277198ff8b6f785478aa9a39f403cb768dd02cbee326c3e7da348845f") {
        return Some("UniswapV2");
    }
    // Uniswap V3
    if *code_hash == b256!("1a4aa08e8e074a1a1d2c7616f0f1a7a8f5e74cb701c481360a3b16c6c0c51d64") {
        return Some("UniswapV3");
    }
    // SushiSwap V2
    if *code_hash == b256!("d0db06a3d0b7e5b5cbc2a19afe5cb869e2b5e904d7130be5ad5e7616fc82ed02") {
        return Some("SushiSwapV2");
    }
    // PancakeSwap V2
    if *code_hash == b256!("d0d4c4cd0848c93cb4fd1f498d7013ee6bfb25783b93e2e8b7d1f0e0b0a0d01e") {
        return Some("PancakeSwapV2");
    }
    None
}

pub fn is_known_factory(code_hash: &B256) -> bool {
    identify_factory(code_hash).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identify_known() {
        let univ2 = b256!("96e8ac4277198ff8b6f785478aa9a39f403cb768dd02cbee326c3e7da348845f");
        assert_eq!(identify_factory(&univ2), Some("UniswapV2"));

        let univ3 = b256!("1a4aa08e8e074a1a1d2c7616f0f1a7a8f5e74cb701c481360a3b16c6c0c51d64");
        assert_eq!(identify_factory(&univ3), Some("UniswapV3"));
    }

    #[test]
    fn test_unknown_codehash() {
        let unknown = b256!("0000000000000000000000000000000000000000000000000000000000000000");
        assert!(!is_known_factory(&unknown));
    }
}
