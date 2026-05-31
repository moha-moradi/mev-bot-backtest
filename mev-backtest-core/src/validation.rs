use std::collections::HashSet;

use crate::config::{ChainConfig, Config};
use crate::types::{
    ChainName, FlashLoanProvider, GasModel, OutputFormat, RangeMode, Strategy,
};

#[derive(Debug)]
pub struct ValidationResult {
    pub chain_name: ChainName,
    pub chain_config: ChainConfig,
    pub range_mode: RangeMode,
    pub strategies: Vec<Strategy>,
    pub flash_loan_provider: FlashLoanProvider,
    pub parallelism: usize,
}

#[derive(Debug)]
pub enum ValidationError {
    Message(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::Message(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ValidationError {}

fn count_set_flags(cfg: &Config) -> Vec<&'static str> {
    let mut flags = Vec::new();
    if cfg.days.is_some() {
        flags.push("--days");
    }
    if cfg.blocks.is_some() {
        flags.push("--blocks");
    }
    if cfg.block.is_some() {
        flags.push("--block");
    }
    if cfg.from_block.is_some() || cfg.to_block.is_some() {
        flags.push("--from-block/--to-block");
    }
    flags
}

fn check_range_conflicts(cfg: &Config) -> Result<RangeMode, ValidationError> {
    let active = count_set_flags(cfg);

    if active.len() > 1 {
        return Err(ValidationError::Message(format!(
            "Error: {} cannot be used together.\n\
             Use exactly one of: --days, --blocks, --block, or --from-block/--to-block.",
            active.join(" and ")
        )));
    }

    // Check from/to pairing
    let from = cfg.from_block;
    let to = cfg.to_block;

    if (from.is_some() && to.is_none()) || (from.is_none() && to.is_some()) {
        return Err(ValidationError::Message(
            "Error: --from-block and --to-block must be used together.".to_string(),
        ));
    }

    if let (Some(f), Some(t)) = (from, to) {
        if t <= f {
            return Err(ValidationError::Message(format!(
                "Error: --to-block ({t}) must be greater than --from-block ({f})."
            )));
        }
        return Ok(RangeMode::Range(f, t));
    }

    if let Some(d) = cfg.days {
        if d < 1 || d > 365 {
            return Err(ValidationError::Message(
                "Error: --days must be between 1 and 365.".to_string(),
            ));
        }
        return Ok(RangeMode::Days(d));
    }

    if let Some(b) = cfg.blocks {
        if b < 1 {
            return Err(ValidationError::Message(
                "Error: --blocks must be >= 1.".to_string(),
            ));
        }
        return Ok(RangeMode::Blocks(b));
    }

    if let Some(b) = cfg.block {
        if b == 0 {
            return Err(ValidationError::Message(
                "Error: --block must be > 0.".to_string(),
            ));
        }
        return Ok(RangeMode::Single(b));
    }

    Err(ValidationError::Message(
        "Error: no block range specified.\n\
         Use one of: --days, --blocks, --block, or --from-block + --to-block."
            .to_string(),
    ))
}

/// Validates config for the replay subcommand.
/// Only allows --block (single block), rejects all other range flags.
pub fn validate_replay(config: &Config) -> Result<(ChainName, ChainConfig), ValidationError> {
    let chain_name: ChainName = config
        .chain
        .parse()
        .map_err(|e: String| ValidationError::Message(format!("Error: {e}")))?;

    let chain_config = config
        .chains
        .get(chain_name.to_string().as_str())
        .cloned()
        .ok_or_else(|| {
            ValidationError::Message(format!(
                "Error: no [chains.{}] section found in config.",
                chain_name
            ))
        })?;

    // Replay only supports --block
    let active = count_set_flags(config);
    if active.len() > 1 {
        return Err(ValidationError::Message(format!(
            "Error: {} cannot be used together.\n\
             Use exactly one of: --days, --blocks, --block, or --from-block/--to-block.",
            active.join(" and ")
        )));
    }

    let from = config.from_block;
    let to = config.to_block;
    if (from.is_some() && to.is_none()) || (from.is_none() && to.is_some()) {
        return Err(ValidationError::Message(
            "Error: --from-block and --to-block must be used together.".to_string(),
        ));
    }

    // Check for non-block range flags — replay only supports --block
    if config.days.is_some() {
        return Err(ValidationError::Message(
            "Error: --days is not supported by the replay subcommand. Use --block instead.".to_string(),
        ));
    }
    if config.blocks.is_some() {
        return Err(ValidationError::Message(
            "Error: --blocks is not supported by the replay subcommand. Use --block instead.".to_string(),
        ));
    }
    if config.from_block.is_some() || config.to_block.is_some() {
        return Err(ValidationError::Message(
            "Error: --from-block/--to-block is not supported by the replay subcommand. Use --block instead.".to_string(),
        ));
    }
    if config.block.is_none() || config.block == Some(0) {
        return Err(ValidationError::Message(
            "Error: --block is required for the replay subcommand and must be > 0.".to_string(),
        ));
    }

    // Validate RPC URL
    if let Some(url) = &config.rpc_url {
        if url.trim().is_empty() {
            return Err(ValidationError::Message(
                "Error: --rpc URL cannot be empty.".to_string(),
            ));
        }
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ValidationError::Message(format!(
                "Error: --rpc URL '{}' must start with http:// or https://.",
                url
            )));
        }
    }

    Ok((chain_name, chain_config))
}

pub fn validate_and_resolve(config: &Config) -> Result<ValidationResult, ValidationError> {
    validate_and_resolve_for(config, true)
}

pub fn validate_and_resolve_for(config: &Config, check_strategies: bool) -> Result<ValidationResult, ValidationError> {
    // 1. Parse and validate chain name
    let chain_name: ChainName = config
        .chain
        .parse()
        .map_err(|e: String| ValidationError::Message(format!("Error: {e}")))?;

    // 2. Check chain config exists
    let chain_config = config
        .chains
        .get(chain_name.to_string().as_str())
        .cloned()
        .ok_or_else(|| {
            ValidationError::Message(format!(
                "Error: no [chains.{}] section found in config.",
                chain_name
            ))
        })?;

    // 3. Parse and validate flash loan provider
    let provider: FlashLoanProvider = config.flash_loan_provider.parse().map_err(|e: String| {
        ValidationError::Message(format!("Error: {e}"))
    })?;

    if provider.is_forced() {
        let contract_field = match provider {
            FlashLoanProvider::Balancer => "balancer_vault",
            FlashLoanProvider::Aave => "aave_v3_pool",
            FlashLoanProvider::Uniswap => "uniswap_v3_factory",
            _ => unreachable!(),
        };
        let has_contract = match provider {
            FlashLoanProvider::Balancer => chain_config.balancer_vault.is_some(),
            FlashLoanProvider::Aave => chain_config.aave_v3_pool.is_some(),
            FlashLoanProvider::Uniswap => chain_config.uniswap_v3_factory.is_some(),
            _ => true,
        };
        if !has_contract {
            tracing::warn!(
                "{} contract address is missing for chain '{}'. \
                 Opportunities requiring this provider will be SKIPPED_NO_FLASHLOAN.",
                contract_field,
                chain_name
            );
        }
    }

    // 4. Parse and validate strategies (skip for fetch/report subcommands)
    let strategies: Vec<Strategy> = if check_strategies {
        let mut s = Strategy::from_comma_list(&config.strategies)
            .map_err(|e| ValidationError::Message(format!("Error: {e}")))?;

        // Check for JIT/JIT+Arb with no CL pools
        let has_cl_pools = {
            if let Some(reg_path) = &chain_config.pools_registry_path {
                if let Ok(content) = std::fs::read_to_string(reg_path) {
                    if let Ok(pools) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                        pools.iter().any(|p| {
                            p.get("is_concentrated_liquidity")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false)
                        })
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        };

        let jit_strategies: HashSet<Strategy> = [Strategy::Jit, Strategy::JitArb].into();
        let has_jit = s.iter().any(|st| jit_strategies.contains(st));

        if has_jit && !has_cl_pools {
            tracing::warn!(
                "No concentrated liquidity pools found for chain '{}'. \
                 JIT strategy requires Uniswap V3 fork pools. Skipping JIT/JIT_Arb.",
                chain_name
            );
            s.retain(|st| !jit_strategies.contains(st));
        }
        s
    } else {
        Vec::new()
    };

    // 5. Validate block range
    let range_mode = check_range_conflicts(config)?;

    // 6. Validate parallelism
    let parallelism = match config.parallelism {
        Some(n) if n < 1 => {
            return Err(ValidationError::Message(
                "Error: --parallelism must be >= 1.".to_string(),
            ));
        }
        Some(n) => n as usize,
        None => std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4),
    };

    // 7. Validate coinbase bribe
    if config.coinbase_bribe > 100 {
        return Err(ValidationError::Message(
            "Error: --coinbase-bribe must be between 0 and 100.".to_string(),
        ));
    }

    // 8. Validate priority fee
    if config.priority_fee < 0.0 {
        return Err(ValidationError::Message(
            "Error: --priority-fee must be >= 0.".to_string(),
        ));
    }

    // 9. Validate RPC URL
    if let Some(url) = &config.rpc_url {
        if url.trim().is_empty() {
            return Err(ValidationError::Message(
                "Error: --rpc URL cannot be empty.".to_string(),
            ));
        }
        // Basic URL validation
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ValidationError::Message(format!(
                "Error: --rpc URL '{}' must start with http:// or https://.",
                url
            )));
        }
    }

    // 10. Validate gas model
    let _gas_model: GasModel = config.gas_model.parse().map_err(|e: String| {
        ValidationError::Message(format!("Error: {e}"))
    })?;

    // 11. Validate output format
    let _output: OutputFormat = config.output.parse().map_err(|e: String| {
        ValidationError::Message(format!("Error: {e}"))
    })?;

    Ok(ValidationResult {
        chain_name,
        chain_config,
        range_mode,
        strategies,
        flash_loan_provider: provider,
        parallelism,
    })
}
