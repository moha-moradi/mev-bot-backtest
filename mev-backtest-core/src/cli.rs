use clap::{Args, Parser, Subcommand};

/// MEV Bot Backtest Engine — high-fidelity historical backtest
/// for EVM-compatible chains.
#[derive(Parser, Debug)]
#[command(name = "mev-backtest", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Path to TOML config file
    #[arg(global = true, short = 'f', long = "config", value_name = "FILE")]
    pub config: Option<String>,

    /// Enable debug-level logging
    #[arg(global = true, short, long)]
    pub verbose: bool,

    /// Suppress all output except the final summary
    #[arg(global = true, long)]
    pub quiet: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Execute the full backtest
    Run(RunArgs),

    /// Pre-cache block data without running strategies
    Fetch(FetchArgs),

    /// Re-render terminal tables from saved JSON
    Report,

    /// Print the fully resolved config as TOML
    Config,

    /// Replay a specific block for debugging (not yet implemented)
    Replay(ReplayArgs),
}

#[derive(Args, Debug, Clone)]
#[command(next_help_heading = "Block Range (exactly one required)")]
pub struct BlockRangeArgs {
    /// Last N days of blocks (1–365)
    #[arg(long, value_name = "N")]
    pub days: Option<u64>,

    /// Last N blocks from chain tip (≥1)
    #[arg(long, value_name = "N")]
    pub blocks: Option<u64>,

    /// Single specific block number (>0)
    #[arg(long, value_name = "NUMBER")]
    pub block: Option<u64>,

    /// Range start (requires --to-block)
    #[arg(long, value_name = "NUMBER")]
    pub from_block: Option<u64>,

    /// Range end (requires --from-block)
    #[arg(long, value_name = "NUMBER")]
    pub to_block: Option<u64>,
}

#[derive(Args, Debug, Clone)]
#[command(next_help_heading = "Chain & Connection")]
pub struct ChainArgs {
    /// Chain name: polygon, avalanche, bsc, arbitrum, base
    #[arg(short = 'n', long, default_value = "polygon", value_name = "NAME")]
    pub chain: String,

    /// Archive node RPC endpoint
    #[arg(short = 'r', long = "rpc", value_name = "URL")]
    pub rpc_url: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct RunArgs {
    #[command(flatten)]
    pub block_range: BlockRangeArgs,

    #[command(flatten)]
    pub chain_args: ChainArgs,

    /// Flash loan provider strategy: auto, balancer, aave, uniswap
    #[arg(long, default_value = "auto", value_name = "PROVIDER", help_heading = "Flash Loan")]
    pub flash_loan_provider: String,

    /// Strategies to run: comma-separated or "all"
    #[arg(long, default_value = "all", value_name = "LIST", help_heading = "Strategies")]
    pub strategies: String,

    /// Gas price model: historical_exact, p90, fixed
    #[arg(long, default_value = "historical_exact", value_name = "MODEL", help_heading = "Gas Model")]
    pub gas_model: String,

    /// Premium added in historical_exact mode (gwei)
    #[arg(long, default_value_t = 1.0, value_name = "GWEI", help_heading = "Gas Model")]
    pub priority_fee: f64,

    /// Percentage of gross profit as validator tip (0–100)
    #[arg(long, default_value_t = 10, value_name = "PERCENT", help_heading = "Gas Model")]
    pub coinbase_bribe: u8,

    /// Minimum profit threshold in USD for verbose output
    #[arg(long, default_value_t = 0.0, value_name = "USD", help_heading = "Output")]
    pub min_profit_usd: f64,

    /// Output format: table, csv, json
    #[arg(long, default_value = "table", value_name = "FORMAT", help_heading = "Output")]
    pub output: String,

    /// Directory for CSV/JSON exports
    #[arg(long, default_value = "./results", value_name = "PATH", help_heading = "Output")]
    pub export_path: String,

    /// Block/state cache directory
    #[arg(long, default_value = "./cache", value_name = "PATH", help_heading = "Output")]
    pub cache_dir: String,

    /// Concurrent block workers (default: CPU core count)
    #[arg(long, value_name = "N", help_heading = "Output")]
    pub parallelism: Option<u64>,
}

#[derive(Args, Debug, Clone)]
pub struct FetchArgs {
    #[command(flatten)]
    pub block_range: BlockRangeArgs,

    #[command(flatten)]
    pub chain_args: ChainArgs,

    /// Concurrent block workers (default: CPU core count)
    #[arg(long, value_name = "N")]
    pub parallelism: Option<u64>,
}

#[derive(Args, Debug, Clone)]
pub struct ReplayArgs {
    /// Block number to replay (required)
    #[arg(long, required = true, value_name = "NUMBER")]
    pub block: u64,

    /// Replay up to this tx index (default: all)
    #[arg(long, value_name = "INDEX")]
    pub tx_index: Option<usize>,

    #[command(flatten)]
    pub chain_args: ChainArgs,

    /// Block/state cache directory
    #[arg(long, default_value = "./cache", value_name = "PATH")]
    pub cache_dir: String,
}
