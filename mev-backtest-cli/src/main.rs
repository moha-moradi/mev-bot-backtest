use clap::Parser;
use mev_backtest_core::{cli::{BlockRangeArgs, Cli, Commands}, config::Config, Result};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up tracing subscriber
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    if !cli.quiet {
        fmt().with_env_filter(filter).init();
    }

    // Load config
    let config = if let Some(ref path) = cli.config {
        Config::load_toml(path)?
    } else {
        Config::default()
    };

    // Validate CLI
    cli.validate()?;

    // Resolve config with CLI overrides
    let resolved = cli.resolve_config(&config)?;
    resolved.validate()?;

    match cli.command {
        Some(Commands::Config) => handle_config(cli.quiet, &resolved),
        Some(Commands::Fetch { range }) => handle_fetch(cli.quiet, &resolved, &range, &cli.cache_dir),
        Some(Commands::Replay { block, tx_index }) => handle_replay(cli.quiet, &resolved, block, tx_index),
        Some(Commands::Report) => handle_report(cli.quiet, &resolved),
        None => handle_run(cli.quiet, &resolved, &cli),
    }
}

fn handle_config(quiet: bool, config: &Config) -> Result<()> {
    if !quiet {
        println!("{}", config.to_toml()?);
    }
    Ok(())
}

fn handle_fetch(quiet: bool, config: &Config, _range: &BlockRangeArgs, cache_dir: &std::path::Path) -> Result<()> {
    if !quiet {
        println!("[DRY RUN] Fetch command would resolve range and download block data");
        println!("Chain: {}", config.chain);
        if let Some(ref rpc_url) = config.rpc_url {
            println!("RPC: {}", rpc_url);
        }
        println!("Cache Dir: {}", cache_dir.display());
    }
    Ok(())
}

fn handle_replay(quiet: bool, config: &Config, block: u64, tx_index: Option<usize>) -> Result<()> {
    if !quiet {
        println!("[DRY RUN] Replay command would replay state");
        println!("Chain: {}", config.chain);
        println!("Block: {}", block);
        if let Some(idx) = tx_index {
            println!("Tx Index: {}", idx);
        }
    }
    Ok(())
}

fn handle_report(quiet: bool, config: &Config) -> Result<()> {
    if !quiet {
        println!("[DRY RUN] Report command would load and display previous results");
        println!("Chain: {}", config.chain);
    }
    Ok(())
}

fn handle_run(quiet: bool, config: &Config, cli: &Cli) -> Result<()> {
    if !quiet {
        println!("========================================");
        println!("         MEV BACKTEST ENGINE");
        println!("========================================");
        println!();
        println!("Chain: {}", config.chain);
        if let Some(ref rpc_url) = config.rpc_url {
            println!("RPC: {}", rpc_url);
        } else {
            println!("RPC: (not configured)");
        }

        match &config.range_mode {
            Some(mev_backtest_core::config::RangeMode::Days(days)) => {
                println!("Block Range: --days {} (resolve at runtime)", days);
            }
            Some(mev_backtest_core::config::RangeMode::Blocks(blocks)) => {
                println!("Block Range: --blocks {} (resolve at runtime)", blocks);
            }
            Some(mev_backtest_core::config::RangeMode::Block(block)) => {
                println!("Block Range: --block {} (single block)", block);
            }
            Some(mev_backtest_core::config::RangeMode::FromTo { from_block, to_block }) => {
                let count = to_block - from_block + 1;
                println!("Block Range: --from-block {} --to-block {} ({} blocks)", from_block, to_block, count);
            }
            None => {
                eprintln!("Error: Please specify one of --days, --blocks, --block, or --from-block/--to-block");
                std::process::exit(1);
            }
        }

        println!("Strategies: {}", config.strategies.join(", "));
        println!("Flash Loan Provider: {:?}", config.flash_loan_provider);
        println!("Bribe: {}%", config.gas_model.bribe_pct);
        println!("Priority Fee: {} wei", config.gas_model.priority_fee);
        println!("Parallelism: {}", config.gas_model.parallelism);
        println!("Cache Dir: {}", cli.cache_dir.display());
        println!();
    }
    
    println!("[DRY RUN] No blockchain calls will be made");
    Ok(())
}