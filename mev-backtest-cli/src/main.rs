use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use tracing_subscriber::EnvFilter;

use mev_backtest_core::cache::{CacheStore, RunManifest};
use mev_backtest_core::cli::{Cli, Command};
use mev_backtest_core::config::CliOverrides;
use mev_backtest_core::fetch::Fetcher;
use mev_backtest_core::replay::BlockReplayer;
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
        Command::Replay(args) => CliOverrides {
            days: None,
            blocks: None,
            block: Some(args.block),
            from_block: None,
            to_block: None,
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
            cache_dir: Some(args.cache_dir.clone()),
            parallelism: None,
        },
        Command::Report | Command::Config => CliOverrides {
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
            let rpc_url = config.effective_rpc_url(validation_result.chain_name);
            let rpc = RpcClient::new(&rpc_url, validation_result.chain_config.chain_id)?;

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
        Command::Replay(args) => {
            let (chain_name, chain_config) = match validation::validate_replay(&config) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };

            let rpc_url = config.effective_rpc_url(chain_name);
            let rpc = RpcClient::new(&rpc_url, chain_config.chain_id)?;
            let cache = CacheStore::open(&config.cache_dir, chain_config.chain_id)?;

            let block_num = args.block;
            let tx_index = args.tx_index.unwrap_or(usize::MAX);

            // Verify block is cached
            if !cache.has_block(block_num)? {
                eprintln!(
                    "Error: Block {} is not cached. Run `mev-backtest fetch --block {}` first.",
                    block_num, block_num
                );
                std::process::exit(1);
            }

            let replayer = BlockReplayer::new(
                tokio::runtime::Handle::current(),
                cache,
                rpc,
                chain_config.chain_id,
            );
            let txs = replayer
                .load_txs(block_num)
                .map_err(|e| anyhow::anyhow!("Failed to load txs for block {}: {}", block_num, e))?;
            let actual_count = txs.len();
            let end_tx = tx_index.min(actual_count.saturating_sub(1));

            println!(
                "Replaying block {} on chain {} ({} txs, replaying 0..{})",
                block_num, chain_name, actual_count, end_tx
            );
            println!();

            let start = std::time::Instant::now();
            let (_snapshot, results) = replayer
                .replay_to(block_num, end_tx)
                .map_err(|e| anyhow::anyhow!("Replay failed for block {}: {}", block_num, e))?;
            let elapsed = start.elapsed();

            println!(
                "  {:<4} {:<66} {:<6} {:<8} {}",
                "idx", "tx_hash", "status", "gas_used", "receipt"
            );
            println!("  {}", "─".repeat(100));

            let mut matched = 0u64;
            let mut total = 0u64;

            for r in &results {
                let status_str = if r.status { "ok" } else { "fail" };
                let receipt_str = match &r.error {
                    None => {
                        matched += 1;
                        "✓".to_string()
                    }
                    Some(_) => "✗".to_string(),
                };
                total += 1;

                println!(
                    "  {:<4} {:<66} {:<6} {:<8} {}",
                    r.index, r.tx_hash, status_str, r.gas_used, receipt_str
                );
            }

            println!();
            let pct = if total > 0 {
                (matched as f64 / total as f64) * 100.0
            } else {
                100.0
            };
            println!(
                "  Receipt verification: {}/{} match ({:.1}%) — {:.2}s",
                matched, total, pct, elapsed.as_secs_f64()
            );

            if pct < 99.0 {
                tracing::warn!(
                    "Receipt match rate {:.1}% is below 99% threshold",
                    pct
                );
            }
        }
    }

    Ok(())
}





