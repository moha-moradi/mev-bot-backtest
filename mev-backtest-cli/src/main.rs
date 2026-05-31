use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use tracing_subscriber::EnvFilter;

use mev_backtest_core::cache::{CacheStore, RunManifest};
use mev_backtest_core::cli::{Cli, Command};
use mev_backtest_core::config::CliOverrides;
use mev_backtest_core::fetch::Fetcher;
use mev_backtest_core::resolver::RangeResolver;
use mev_backtest_core::rpc::RpcClient;
use mev_backtest_core::validation;

fn setup_logging(verbose: bool, quiet: bool) {
    let filter = if quiet {
        EnvFilter::new("error")
    } else if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .without_time()
        .with_target(false)
        .init();
}

fn build_overrides(cli: &Cli) -> CliOverrides {
    match &cli.command {
        Command::Run(args) => CliOverrides {
            days: args.block_range.days,
            blocks: args.block_range.blocks,
            block: args.block_range.block,
            from_block: args.block_range.from_block,
            to_block: args.block_range.to_block,
            chain: Some(args.chain_args.chain.clone()),
            rpc_url: args.chain_args.rpc_url.clone(),
            flash_loan_provider: Some(args.flash_loan_provider.clone()),
            strategies: Some(args.strategies.clone()),
            gas_model: Some(args.gas_model.clone()),
            priority_fee: Some(args.priority_fee),
            coinbase_bribe: Some(args.coinbase_bribe),
            min_profit_usd: Some(args.min_profit_usd),
            output: Some(args.output.clone()),
            export_path: Some(args.export_path.clone()),
            cache_dir: Some(args.cache_dir.clone()),
            parallelism: args.parallelism,
        },
        Command::Fetch(args) => CliOverrides {
            days: args.block_range.days,
            blocks: args.block_range.blocks,
            block: args.block_range.block,
            from_block: args.block_range.from_block,
            to_block: args.block_range.to_block,
            chain: Some(args.chain_args.chain.clone()),
            rpc_url: args.chain_args.rpc_url.clone(),
            flash_loan_provider: None,
            strategies: None,
            gas_model: None,
            priority_fee: None,
            coinbase_bribe: None,
            min_profit_usd: None,
            output: None,
            export_path: None,
            cache_dir: None,
            parallelism: args.parallelism,
        },
        Command::Report | Command::Config | Command::Replay(_) => CliOverrides {
            days: None,
            blocks: None,
            block: None,
            from_block: None,
            to_block: None,
            chain: None,
            rpc_url: None,
            flash_loan_provider: None,
            strategies: None,
            gas_model: None,
            priority_fee: None,
            coinbase_bribe: None,
            min_profit_usd: None,
            output: None,
            export_path: None,
            cache_dir: None,
            parallelism: None,
        },
    }
}

fn print_startup_plan(result: &validation::ValidationResult, config: &mev_backtest_core::config::Config) {
    let divider = "═".repeat(55);

    println!();
    println!("  ╔{divider}╗");
    println!("  ║        MEV Backtest Engine — Startup Plan        ║");
    println!("  ╚{divider}╝");
    println!();

    let plan = config.plan_summary(
        result.chain_name,
        &result.chain_config,
        &result.range_mode,
        &result.strategies,
        result.flash_loan_provider,
    );

    for line in plan.lines() {
        println!("  {line}");
    }

    println!("  [DRY RUN — no simulation yet]");
    println!();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    setup_logging(cli.verbose, cli.quiet);

    // Load config
    let config_path = cli.config.as_deref().unwrap_or("mev-backtest.toml");
    let mut config = if Path::new(config_path).exists() {
        mev_backtest_core::config::Config::load(config_path)?
    } else {
        let mut cfg = mev_backtest_core::config::Config::default();
        cfg.config_path = Some(std::path::PathBuf::from(config_path));
        cfg
    };

    // Merge CLI overrides
    let overrides = build_overrides(&cli);
    config.merge_cli(&overrides);

    // Dispatch
    match &cli.command {
        Command::Run(_) => {
            let validation_result = match validation::validate_and_resolve(&config) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };
            print_startup_plan(&validation_result, &config);
        }
        Command::Fetch(_) => {
            let validation_result = match validation::validate_and_resolve_for(&config, false) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };

            // Build RPC client
            let rpc_url = config.rpc_url.as_deref().ok_or_else(|| {
                anyhow::anyhow!("RPC URL is required. Use --rpc <URL> or set rpc_url in config.")
            })?;
            let rpc = RpcClient::new(rpc_url, validation_result.chain_config.chain_id)?;

            // Open cache
            let cache = CacheStore::open(&config.cache_dir, validation_result.chain_config.chain_id)?;

            // Resolve block range
            let resolver = RangeResolver::new(rpc.clone());
            let resolved = resolver.resolve(&validation_result.range_mode).await?;

            let run_id = format!(
                "run_{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            );

            // Create and store manifest
            let manifest = RunManifest {
                run_id: run_id.clone(),
                chain: validation_result.chain_name.to_string(),
                start_block: resolved.start_block,
                end_block: resolved.end_block,
                resolved_at: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                range_mode: resolved.mode_string(),
                strategies: vec![],
                flash_loan_provider: String::new(),
            };
            cache.put_manifest(&manifest)?;

            println!("Run ID: {}", run_id);
            println!("{}", resolved.summary());
            println!();

            // Fetch blocks
            let parallel = config.parallelism.unwrap_or(0) as usize;
            let fetcher = if parallel > 0 {
                Fetcher::new(rpc, cache).with_parallelism(parallel)
            } else {
                Fetcher::new(rpc, cache)
            };

            let pb = ProgressBar::new(resolved.block_count);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} blocks ({eta})")?
                    .progress_chars("=> "),
            );

            let tick = || pb.tick();
            let summary = fetcher.fetch_range(&resolved, Some(&tick)).await?;
            pb.finish_and_clear();

            println!();
            println!("Fetch complete:");
            println!("  Total blocks: {}", summary.total_blocks);
            println!("  Fetched:      {}", summary.fetched);
            println!("  Cached:       {}", summary.cached);
            println!("  Elapsed:      {:.2}s", summary.elapsed_secs);

            // Integrity check
            if !summary.missing_after_fetch.is_empty() {
                println!(
                    "  Missing:      {} blocks — auto-refetching...",
                    summary.missing_after_fetch.len()
                );
                let refetched = fetcher
                    .auto_refetch_gaps(&summary.missing_after_fetch)
                    .await?;
                println!("  Refetched:    {}", refetched);
            }
        }
        Command::Report => {
            println!("report subcommand — not yet implemented");
        }
        Command::Config => {
            let toml_str = config.to_toml_string()?;
            println!("{}", toml_str);
        }
        Command::Replay(_) => {
            println!("replay subcommand — not yet implemented");
        }
    }

    Ok(())
}
