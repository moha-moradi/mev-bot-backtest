use crate::config::*;
use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "mev-backtest")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[command(flatten)]
    pub range: BlockRangeArgs,

    #[command(flatten)]
    pub chain_args: ChainArgs,

    #[command(flatten)]
    pub flash_loan_args: FlashLoanArgs,

    #[command(flatten)]
    pub strategy_args: StrategyArgs,

    #[command(flatten)]
    pub gas_args: GasArgs,

    #[command(flatten)]
    pub output_args: OutputArgs,

    /// Config file path
    #[arg(short, long, env = "MEV_CONFIG")]
    pub config: Option<std::path::PathBuf>,

    /// Cache directory for block data
    #[arg(short, long, default_value = "./cache")]
    pub cache_dir: std::path::PathBuf,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Suppress output except final summary
    #[arg(short, long)]
    pub quiet: bool,
}

#[derive(Debug, Args)]
#[group(required = false, multiple = false)]
pub struct BlockRangeArgs {
    /// Number of days to backtest (from current tip)
    #[arg(long, value_name = "DAYS", conflicts_with_all = ["blocks", "single_block", "from_block", "to_block"])]
    pub days: Option<u64>,

    /// Number of blocks to backtest (from current tip)
    #[arg(long, value_name = "BLOCKS", conflicts_with_all = ["days", "single_block", "from_block", "to_block"])]
    pub blocks: Option<u64>,

    /// Single block number to analyze
    #[arg(long, value_name = "NUMBER", conflicts_with_all = ["days", "blocks", "from_block", "to_block"])]
    pub block: Option<u64>,

    /// Starting block number
    #[arg(long, value_name = "NUMBER", requires = "to_block")]
    pub from_block: Option<u64>,

    /// Ending block number
    #[arg(long, value_name = "NUMBER", requires = "from_block")]
    pub to_block: Option<u64>,
}

#[derive(Debug, Args)]
pub struct ChainArgs {
    /// Chain to backtest (ethereum, polygon, arbitrum, optimism)
    #[arg(short, long, default_value = "ethereum")]
    pub chain: String,

    /// RPC URL (overrides config file)
    #[arg(short, long, env = "MEV_RPC_URL")]
    pub rpc_url: Option<String>,
}

#[derive(Debug, Args)]
pub struct FlashLoanArgs {
    /// Flash loan provider (auto, balancer_v2, aave_v3, uniswap_swap)
    #[arg(long, default_value = "auto")]
    pub flash_loan_provider: String,
}

#[derive(Debug, Args)]
pub struct StrategyArgs {
    /// Strategies to run (two_hop_arb, multi_hop_arb, jit, jit_arb, sandwich)
    #[arg(short, long, default_values = ["two_hop_arb", "multi_hop_arb"])]
    pub strategies: Vec<String>,
}

#[derive(Debug, Args)]
pub struct GasArgs {
    /// Coinbase bribe percentage (0.0-100.0)
    #[arg(long, default_value = "0.0")]
    pub bribe_pct: f64,

    /// Priority fee in wei (default: 1_000_000 = 0.001 gwei)
    #[arg(long, default_value = "1000000")]
    pub priority_fee: u64,

    /// Parallelism level for block fetching
    #[arg(long, default_value = "4")]
    pub parallelism: u64,
}

#[derive(Debug, Args)]
pub struct OutputArgs {
    /// Export results to CSV files
    #[arg(long)]
    pub csv: bool,

    /// Export results to JSON file
    #[arg(long)]
    pub json: bool,

    /// Minimum profit USD to include in verbose output
    #[arg(long, default_value = "0.0")]
    pub min_profit_usd: f64,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Print fully resolved configuration
    Config,
    /// Fetch block data without running strategies
    Fetch {
        #[command(flatten)]
        range: BlockRangeArgs,
    },
    /// Replay block state for debugging
    Replay {
        /// Block number to replay
        #[arg(short, long)]
        block: u64,

        /// Transaction index to replay to
        #[arg(short, long)]
        tx_index: Option<usize>,
    },
    /// Generate report from previous run
    Report,
}

impl Cli {
    pub fn resolve_config(&self, config: &Config) -> anyhow::Result<Config> {
        let mut resolved = config.clone();

        // Override chain
        resolved.chain = self.chain_args.chain.parse()?;

        // Override RPC URL
        if let Some(ref url) = self.rpc_url {
            resolved.rpc_url = Some(url.clone());
        }

        // Override flash loan provider
        resolved.flash_loan_provider = self.flash_loan_args.flash_loan_provider.parse()?;

        // Override strategies
        resolved.strategies = self.strategy_args.strategies.clone();

        // Override gas model
        resolved.gas_model = GasModel {
            bribe_pct: self.gas_args.bribe_pct,
            priority_fee: self.gas_args.priority_fee,
            parallelism: self.gas_args.parallelism,
        };

        // Override output
        resolved.output = OutputConfig {
            csv: self.output_args.csv,
            json: self.output_args.json,
            min_profit_usd: self.output_args.min_profit_usd,
        };

        // Set range mode
        resolved.range_mode = self.resolve_range_mode()?;

        Ok(resolved)
    }

    fn resolve_range_mode(&self) -> anyhow::Result<Option<RangeMode>> {
        match (&self.range.days, &self.range.blocks, &self.range.block) {
            (Some(days), None, None) => Ok(Some(RangeMode::Days(*days))),
            (None, Some(blocks), None) => Ok(Some(RangeMode::Blocks(*blocks))),
            (None, None, Some(block)) => Ok(Some(RangeMode::Block(*block))),
            (None, None, None) => match (&self.range.from_block, &self.range.to_block) {
                (Some(from), Some(to)) => Ok(Some(RangeMode::FromTo {
                    from_block: *from,
                    to_block: *to,
                })),
                _ => Ok(None),
            },
            _ => anyhow::bail!("Only one block range mode can be specified at a time"),
        }
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        let range_mode = self.resolve_range_mode()?;
        
        // Validate strategies
        for strategy in &self.strategy_args.strategies {
            if !crate::supported_strategies().contains(&strategy.as_str()) {
                anyhow::bail!("Unknown strategy '{}'. Supported: {:?}", strategy, crate::supported_strategies());
            }
        }

        // Validate gas model bounds
        if self.gas_args.bribe_pct < 0.0 || self.gas_args.bribe_pct > 100.0 {
            anyhow::bail!("--bribe-pct must be between 0.0 and 100.0");
        }

        if self.gas_args.parallelism == 0 {
            anyhow::bail!("--parallelism must be at least 1");
        }

        // Validate RPC URL format if provided
        if let Some(ref url) = self.rpc_url {
            Url::parse(url)?;
        }

        Ok(())
    }
}