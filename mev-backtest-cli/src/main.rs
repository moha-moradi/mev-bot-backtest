use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;
use comfy_table::Table;
use indicatif::{ProgressBar, ProgressStyle};
use tracing_subscriber::EnvFilter;

use std::collections::HashSet;

use alloy::primitives::Address;
use mev_backtest_core::cache::{CacheStore, RunManifest};
use mev_backtest_core::cli::{Cli, Command};
use mev_backtest_core::config::CliOverrides;
use mev_backtest_core::fetch::Fetcher;

use mev_backtest_core::pool::graph_client::TheGraphClient;
use mev_backtest_core::pool::state::PoolManager;
use mev_backtest_core::replay::BlockReplayer;
use mev_backtest_core::resolver::RangeResolver;
use mev_backtest_core::rpc::RpcClient;
use mev_backtest_core::run::BacktestRunner;
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
            output: Some(args.output.clone()),
            export_path: Some(args.export_path.clone()),
            cache_dir: Some(args.cache_dir.clone()),
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
            output: None,
            export_path: None,
            cache_dir: None,
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
            output: None,
            export_path: None,
            cache_dir: Some(args.cache_dir.clone()),
        },
        Command::GenerateRegistry(_) => CliOverrides {
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
            output: None,
            export_path: None,
            cache_dir: None,
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
            output: None,
            export_path: None,
            cache_dir: None,
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

            let rpc_url = config.effective_rpc_url(validation_result.chain_name);
            let rpc = RpcClient::new(&rpc_url, validation_result.chain_config.chain_id)?;
            let cache = CacheStore::open(&config.cache_dir, validation_result.chain_config.chain_id)?;

            // Resolve block range
            let resolver = RangeResolver::new(rpc.clone());
            let resolved = match resolver.resolve(&validation_result.range_mode).await {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Error: failed to resolve block range: {}", e);
                    std::process::exit(1);
                }
            };

            let run_id = format!(
                "run_{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            );

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
                strategies: validation_result.strategies.iter().map(|s| s.to_string()).collect(),
                flash_loan_provider: validation_result.flash_loan_provider.to_string(),
            };
            cache.put_manifest(&manifest)?;

            println!("Run ID: {}", run_id);
            println!("{}", resolved.summary());
            println!();

            // Build replayer
            let replayer = BlockReplayer::new(
                tokio::runtime::Handle::current(),
                cache,
                rpc.clone(),
                validation_result.chain_config.chain_id,
            );

            // Init pool manager
            let mut pool_manager = PoolManager::new();
            let prev_block = resolved.start_block.saturating_sub(1);
            let registry_path = validation_result.chain_config.pools_registry_path.as_deref();
            if !validation_result.strategies.is_empty() {
                BacktestRunner::init_pools(
                    &mut pool_manager,
                    registry_path,
                    &rpc,
                    prev_block,
                ).await;
            }

            // Run backtest
            let mut runner = BacktestRunner::new(replayer, pool_manager);
            let start = std::time::Instant::now();
            let all_opportunities = runner.run_range(&resolved)?;
            let elapsed = start.elapsed();

            // Print results
            if all_opportunities.is_empty() {
                println!("No MEV opportunities detected in the specified range.");
            } else {
                println!(
                    "\nDetected {} MEV opportunity(ies) in {:.2}s:\n",
                    all_opportunities.len(),
                    elapsed.as_secs_f64()
                );

                let mut table = Table::new();
                table.set_header(vec![
                    "Block", "Tx", "Strategy",
                    "Input", "Profit (token_out)", "Gas (wei)",
                ]);

                for opp in &all_opportunities {
                    table.add_row(vec![
                        format!("{}", opp.block_number),
                        format!("{}", opp.tx_index),
                        format!("{}", opp.strategy),
                        format!("{}", opp.input_amount),
                        format!("{}", opp.expected_profit),
                        format!("{}", opp.gas_cost_wei),
                    ]);
                }

                println!("{table}");
            }
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
            let fetcher = Fetcher::new(rpc, cache);

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
        Command::GenerateRegistry(args) => {
            generate_registry(args).await?;
        }
    }

    Ok(())
}

async fn generate_registry(args: &mev_backtest_core::cli::GenerateRegistryArgs) -> anyhow::Result<()> {
    let chain: mev_backtest_core::types::ChainName = args.chain.parse()
        .map_err(|e: String| anyhow::anyhow!("{}", e))?;
    let chain_name = chain.to_string();

    let output_path = args.output.replace("{chain}", &chain_name);

    let config = mev_backtest_core::config::Config::default();
    let chain_config = config.chains.get(&chain_name)
        .ok_or_else(|| anyhow::anyhow!("Unknown chain: {}", chain_name))?;

    let rpc_url = args.rpc_url.clone()
        .unwrap_or_else(|| chain.public_rpc_url().to_string());
    let rpc = RpcClient::new(&rpc_url, chain_config.chain_id)?;

    // Load any existing pools to avoid duplicates
    let existing: HashSet<Address> = if std::path::Path::new(&output_path).exists() {
        mev_backtest_core::pool::registry::PoolRegistry::load(&output_path)?
            .iter()
            .map(|p| p.address)
            .collect()
    } else {
        HashSet::new()
    };

    let mut all_pools = Vec::new();

    match args.source.as_str() {
        "thegraph" => {
            let api_key = args.graph_api_key.as_deref()
                .ok_or_else(|| anyhow::anyhow!(
                    "TheGraph API key required. Set THEGRAPH_API_KEY env var or pass --graph-api-key"
                ))?;

            let mut client = TheGraphClient::new(api_key.to_string());
            if let Some(v2_url) = &args.graph_v2_url {
                let v3_url = args.graph_v3_url.clone()
                    .unwrap_or_else(|| "".to_string());
                client = client.with_custom_urls(v2_url.clone(), v3_url);
            }

            if args.v2 {
                let v2_factories = chain_config.uniswap_v2_factories.as_ref();
                if let Some(factories) = v2_factories {
                    for factory_str in factories {
                        let factory = factory_str.parse::<Address>()?;
                        let pools = client.fetch_v2_pools(Some(factory), &existing).await?;
                        tracing::info!("TheGraph V2: {} new pools from factory {}", pools.len(), factory_str);
                        all_pools.extend(pools);
                    }
                } else {
                    // No factory filter: fetch all pairs
                    let pools = client.fetch_v2_pools(None, &existing).await?;
                    tracing::info!("TheGraph V2: {} new pools (no factory filter)", pools.len());
                    all_pools.extend(pools);
                }
            }

            if args.v3 {
                let v3_factory = chain_config.uniswap_v3_factory.as_ref()
                    .and_then(|f| f.parse::<Address>().ok());
                let pools = client.fetch_v3_pools(v3_factory, &existing).await?;
                tracing::info!("TheGraph V3: {} new pools", pools.len());
                all_pools.extend(pools);
            }
        }
        "onchain" => {
            let discoverer = mev_backtest_core::pool::discovery::PoolDiscoverer::new(rpc);
            let to_block = args.to_block.unwrap_or(u64::MAX);
            let factories: Vec<Address> = chain_config.uniswap_v2_factories.as_ref()
                .map(|f| f.iter().filter_map(|s| s.parse::<Address>().ok()).collect())
                .unwrap_or_default();
            let pools = discoverer.discover_new_pools(
                &factories,
                args.from_block,
                to_block,
                &existing,
            ).await?;
            tracing::info!("On-chain V2: {} new pools", pools.len());
            all_pools.extend(pools);
        }
        other => anyhow::bail!("Unknown source: {}. Use 'thegraph' or 'onchain'", other),
    }

    if all_pools.is_empty() {
        println!("No new pools discovered for chain '{}'.", chain_name);
        return Ok(());
    }

    // Merge with existing pools
    let mut registry = if std::path::Path::new(&output_path).exists() {
        mev_backtest_core::pool::registry::PoolRegistry::load(&output_path)?
    } else {
        Vec::new()
    };
    registry.extend(all_pools);

    // Write output
    if let Some(parent) = std::path::Path::new(&output_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&registry)?;
    std::fs::write(&output_path, &json)?;
    println!("Pool registry written to '{}' ({} pools)", output_path, registry.len());

    Ok(())
}





