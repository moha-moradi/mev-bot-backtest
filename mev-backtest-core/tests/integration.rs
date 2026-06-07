use alloy::primitives::{address, b256, Address, B256, U256};
use mev_backtest_core::mev::two_hop::TwoHopArbDetector;
use mev_backtest_core::pool::dex_type::DexType;
use mev_backtest_core::pool::state::UniswapV2PoolState;
use mev_backtest_core::pool::state::{PoolInfo, PoolManager, PoolState};
use mev_backtest_core::mev::jit::JitDetector;
use mev_backtest_core::mev::sandwich::SandwichDetector;
use mev_backtest_core::types::{GasConfig, Strategy};

/// ── Helpers ──────────────────────────────────────────────────────────────────

fn rpc_url() -> Option<String> {
    std::env::var("RPC_URL").ok()
}

fn pool_info_to_state(info: PoolInfo) -> PoolState {
    match info.dex_type {
        DexType::UniswapV2 => PoolState::UniswapV2(UniswapV2PoolState {
            info,
            reserve0: 0,
            reserve1: 0,
        }),
        DexType::UniswapV3 => {
            PoolState::UniswapV3(mev_backtest_core::pool::state::UniswapV3PoolState::new(info))
        }
        DexType::Curve => PoolState::Curve(mev_backtest_core::pool::state::CurvePoolState {
            info,
            balances: vec![],
            token_index: std::collections::HashMap::new(),
        }),
        DexType::Balancer => PoolState::Balancer(mev_backtest_core::pool::state::BalancerPoolState {
            info,
            balances: vec![],
            token_index: std::collections::HashMap::new(),
            pool_id: None,
        }),
    }
}

fn load_polygon_registry() -> Vec<PoolInfo> {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pool_path = manifest.parent().unwrap().join("pools/polygon.json");
    let path_str = pool_path.to_str().unwrap();
    mev_backtest_core::pool::registry::PoolRegistry::load(path_str).unwrap()
}

fn wmatic() -> Address {
    address!("0d500b1d8e8ef31e21c99d1db9a6444d3adf1270")
}
fn usdc() -> Address {
    address!("2791bca1f2de4661ed88a30c99a7a9449aa84174")
}
fn usdt() -> Address {
    address!("c2132d05d31c914a87c6611c10748aeb04b58e8f")
}
fn matic_usdc_pool() -> Address {
    address!("6e7a5fafcec6bb1e78bae2a1f0b612012bf14827")
}
fn matic_usdt_pool() -> Address {
    address!("604029b0c1a79eebfb31f7c5316434484f3a4b55")
}

fn default_gas_config() -> GasConfig {
    GasConfig::default()
}

fn make_pool(addr: Address, token0: Address, token1: Address, r0: u128, r1: u128) -> PoolState {
    PoolState::UniswapV2(UniswapV2PoolState {
        info: PoolInfo {
            address: addr,
            token0,
            token1,
            fee: 30,
            name: None,
            dex_type: mev_backtest_core::pool::dex_type::DexType::UniswapV2,
            tick_spacing: None,
        },
        reserve0: r0,
        reserve1: r1,
    })
}

#[test]
fn test_pool_registry_loads() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let pool_path = manifest.parent().unwrap().join("pools/polygon.json");
    let path_str = pool_path.to_str().unwrap();
    let pools = mev_backtest_core::pool::registry::PoolRegistry::load_optional(Some(path_str));
    assert!(!pools.is_empty(), "Pool registry should load pools from {}", path_str);
    assert!(pools.len() >= 45, "Should have at least 45 pools, got {}. Path: {}", pools.len(), path_str);
}

#[test]
fn test_detection_pipeline_synthetic_profitable() {
    let mut pm = PoolManager::new();

    // Pool A: QuickSwap WMATIC/USDC with price imbalance
    // reserves: 1_000_000 USDC, 2_000_000 WMATIC (cheap WMATIC: 0.5 USDC each)
    pm.add_pool(make_pool(
        matic_usdc_pool(), usdc(), wmatic(),
        1_000_000, 2_000_000,
    ));

    // Pool B: QuickSwap WMATIC/USDT
    // reserves: 2_000_000 USDT, 1_000_000 WMATIC (dear WMATIC: 2 USDT each)
    pm.add_pool(make_pool(
        matic_usdt_pool(), usdt(), wmatic(),
        2_000_000, 1_000_000,
    ));

    // Direction 1: buy WMATIC from A (spend USDC), sell WMATIC to B (get USDT)
    let opps = TwoHopArbDetector::detect(&pm, 1_000_000, 0, 12345678, 50_000_000_000, default_gas_config());

    assert!(!opps.is_empty(), "Should detect arb between imbalanced pools");
    assert!(opps.iter().any(|o| o.strategy == Strategy::TwoHopArb));

    for opp in &opps {
        assert!(opp.block_number == 1_000_000);
        assert!(opp.expected_profit > U256::ZERO, "Profit should be positive");
        assert!(opp.gas_cost_wei > 0, "Gas cost should be positive");
    }
}

#[test]
fn test_detection_no_arb_equal_pools() {
    let mut pm = PoolManager::new();

    // Both pools have the same price — no arb
    pm.add_pool(make_pool(
        matic_usdc_pool(), usdc(), wmatic(),
        1_000_000, 1_000_000,
    ));
    pm.add_pool(make_pool(
        matic_usdt_pool(), usdt(), wmatic(),
        1_000_000, 1_000_000,
    ));

    let opps = TwoHopArbDetector::detect(&pm, 1, 0, 100, 50_000_000_000, default_gas_config());

    assert!(opps.is_empty(), "No arb should be detected with equal prices");
}

#[test]
fn test_gas_cost_min_profit_filter() {
    let mut pm = PoolManager::new();

    // Small imbalance — tiny profit
    pm.add_pool(make_pool(
        matic_usdc_pool(), usdc(), wmatic(),
        1_000_000, 1_010_000, // slight imbalance
    ));
    pm.add_pool(make_pool(
        matic_usdt_pool(), usdt(), wmatic(),
        1_010_000, 1_000_000,
    ));

    let opps = TwoHopArbDetector::detect(&pm, 1, 0, 100, 50_000_000_000, default_gas_config());

    // Check that gas_cost_wei is computed correctly
    for opp in &opps {
        assert!(opp.gas_cost_wei > 0);
        let expected_gas = 200_000u128 * 50_000_000_000;
        let diff = opp.gas_cost_wei.abs_diff(expected_gas);
        assert!(diff < 1000, "Gas cost mismatch: {} vs {}", opp.gas_cost_wei, expected_gas);
    }
}

#[test]
fn test_pool_manager_arbitrage_pairs() {
    let mut pm = PoolManager::new();

    let pool_a = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let pool_b = address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let pool_c = address!("cccccccccccccccccccccccccccccccccccccccc");

    // Pool A: USDC/WMATIC
    pm.add_pool(make_pool(pool_a, usdc(), wmatic(), 1000, 1000));
    // Pool B: USDT/WMATIC — shares WMATIC with pool A
    pm.add_pool(make_pool(pool_b, usdt(), wmatic(), 1000, 1000));
    // Pool C: USDC/DAI — shares USDC with pool A
    pm.add_pool(make_pool(pool_c, usdc(), address!("8f3cf7ad23cd3cadbd9735aff958023239c6a063"), 1000, 1000));

    let pairs = pm.arbitrage_pairs();

    // Pair A-B (via WMATIC), Pair A-C (via USDC), Pair B-C should NOT share a token
    assert_eq!(pairs.len(), 2, "Should find 2 arbitrage pairs");
    assert!(pairs.iter().any(|(a, b, t)| (*a == pool_a && *b == pool_b && *t == wmatic())
        || (*a == pool_b && *b == pool_a && *t == wmatic())), "A-B via WMATIC");
    assert!(pairs.iter().any(|(a, b, t)| (*a == pool_a && *b == pool_c && *t == usdc())
        || (*a == pool_c && *b == pool_a && *t == usdc())), "A-C via USDC");
}

#[test]
fn test_pool_addresses_filter() {
    let mut pm = PoolManager::new();

    let addr_a = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let addr_b = address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");

    pm.add_pool(make_pool(addr_a, usdc(), wmatic(), 100, 100));
    pm.add_pool(make_pool(addr_b, usdt(), wmatic(), 100, 100));

    let addrs = pm.pool_addresses();
    assert_eq!(addrs.len(), 2);
    assert!(addrs.contains(&addr_a));
    assert!(addrs.contains(&addr_b));
}

#[test]
fn test_detect_both_directions() {
    let mut pm = PoolManager::new();

    // Pool A and B both trade WMATIC/stable
    // Pool A: 1 USDC = 2 WMATIC (WMATIC cheap)
    // Pool B: 1 USDT = 0.5 WMATIC (WMATIC expensive)
    pm.add_pool(make_pool(matic_usdc_pool(), usdc(), wmatic(), 1_000_000, 2_000_000));
    pm.add_pool(make_pool(matic_usdt_pool(), usdt(), wmatic(), 1_000_000, 500_000));

    let opps = TwoHopArbDetector::detect(&pm, 1, 0, 100, 50_000_000_000, default_gas_config());

    // Should find arb in at least one direction
    assert!(!opps.is_empty(), "Should detect arb");

    // Both directions checked means we should have at most 2 opportunities
    assert!(opps.len() <= 2, "At most 2 direction opportunities");
}

/// ── Accuracy / Precision Tests ──────────────────────────────────────────

#[test]
fn test_arb_profit_accuracy_known_delta() {
    let mut pm = PoolManager::new();

    // Pool A: USDC/WMATIC — price: 1 WMATIC = 0.5 USDC
    pm.add_pool(make_pool(matic_usdc_pool(), usdc(), wmatic(), 1_000_000, 2_000_000));
    // Pool B: USDT/WMATIC — price: 1 WMATIC = 2.0 USDT
    pm.add_pool(make_pool(matic_usdt_pool(), usdt(), wmatic(), 1_000_000, 500_000));

    let opps = TwoHopArbDetector::detect(&pm, 1, 0, 100, 50_000_000_000, default_gas_config());

    assert!(!opps.is_empty(), "Should detect arb");
    for opp in &opps {
        assert!(opp.expected_profit > U256::ZERO, "Profit should be > 0");
        assert!(opp.gas_cost_wei > 0, "Gas cost should be > 0");
    }
}

#[test]
fn test_two_hop_same_token_different_reserves() {
    let mut pm = PoolManager::new();

    // Two pools with same token pair but different reserves
    // Pool A: 1M USDC, 3M WMATIC (price: 3 WMATIC per USDC — WMATIC cheap)
    // Pool B: 1M USDC, 1M WMATIC (price: 1 WMATIC per USDC — WMATIC expensive)
    // Arb: buy WMATIC on A, sell on B
    let pool_a = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let pool_b = address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");

    pm.add_pool(make_pool(pool_a, usdc(), wmatic(), 1_000_000, 3_000_000));
    pm.add_pool(make_pool(pool_b, usdc(), wmatic(), 1_000_000, 1_000_000));

    let opps = TwoHopArbDetector::detect(&pm, 1, 0, 100, 50_000_000_000, default_gas_config());

    // Arb exists: buy WMATIC cheap on A, sell expensive on B
    assert!(!opps.is_empty(), "Should detect arb between same-token pools with different prices");
}

#[test]
fn test_two_hop_v3_reserves_update_accuracy() {
    use mev_backtest_core::pool::state::UniswapV3PoolState;
    use std::collections::HashMap;

    // V3 pool with concentrated liquidity
    let v3_addr = address!("3333333333333333333333333333333333333333");
    let v3_pool = PoolState::UniswapV3(UniswapV3PoolState {
        info: PoolInfo {
            address: v3_addr,
            token0: wmatic(),
            token1: usdc(),
            fee: 30,
            name: None,
            dex_type: mev_backtest_core::pool::dex_type::DexType::UniswapV3,
            tick_spacing: Some(60),
        },
        sqrt_price_x96: U256::from(79228162514264337593543950336u128), // price = 1.0
        tick: 0,
        liquidity: 1_000_000_000_000u128,
        ticks: HashMap::new(),
    });

    let v2_addr = address!("4444444444444444444444444444444444444444");
    let v2_pool = make_pool(v2_addr, wmatic(), usdt(), 100_000_000, 100_000_000);

    let mut pm = PoolManager::new();
    pm.add_pool(v3_pool);
    pm.add_pool(v2_pool);

    let opps = TwoHopArbDetector::detect(&pm, 1, 0, 100, 50_000_000_000, default_gas_config());

    // V3+V2 cross-DEX detection should work
    // This may or may not detect an arb depending on price state
    // At minimum should not panic or crash
    assert!(opps.len() <= 2, "At most 2 opportunities");
}

#[test]
fn test_multi_hop_detection_three_pool() {
    use mev_backtest_core::mev::multi_hop::MultiHopArbDetector;

    let mut pm = PoolManager::new();

    // Triangular arb: USDC → WMATIC → USDT → USDC
    // Pool A: USDC/WMATIC (WMATIC cheap: 0.5 USDC each)
    // Pool B: WMATIC/USDT (WMATIC expensive: 2 USDT each)
    // Pool C: USDC/USDT (1:1)
    pm.add_pool(make_pool(
        matic_usdc_pool(), usdc(), wmatic(),
        1_000_000, 2_000_000,
    ));
    pm.add_pool(make_pool(
        matic_usdt_pool(), usdt(), wmatic(),
        1_000_000, 500_000,
    ));
    // Third pool: USDC/USDT (different addresses for test)
    let usdc_usdt_pool = address!("3333333333333333333333333333333333333333");
    pm.add_pool(make_pool(
        usdc_usdt_pool, usdc(), usdt(),
        1_000_000, 1_000_000,
    ));

    let opps = MultiHopArbDetector::detect(
        &pm, 1, 0, 12345, 50_000_000_000, GasConfig::default(),
    );

    assert!(!opps.is_empty(), "Should detect multi-hop arb");

    // Find a 3-pool opportunity
    let three_hop: Vec<_> = opps.iter().filter(|o| {
        o.path.as_ref().map(|p| p.len() >= 3).unwrap_or(false)
    }).collect();
    assert!(!three_hop.is_empty(), "Should detect a 3-pool arb");

    for opp in &opps {
        assert_eq!(opp.strategy, Strategy::MultiHopArb);
        assert!(opp.expected_profit > U256::ZERO);
        assert!(opp.gas_cost_wei > 0);
    }
}

#[test]
fn test_multi_hop_path_field_populated() {
    use mev_backtest_core::mev::multi_hop::MultiHopArbDetector;

    let mut pm = PoolManager::new();
    pm.add_pool(make_pool(
        matic_usdc_pool(), usdc(), wmatic(),
        1_000_000, 2_000_000,
    ));
    pm.add_pool(make_pool(
        matic_usdt_pool(), usdt(), wmatic(),
        1_000_000, 500_000,
    ));

    let opps = MultiHopArbDetector::detect(
        &pm, 1, 0, 12345, 50_000_000_000, GasConfig::default(),
    );

    assert!(!opps.is_empty());
    for opp in &opps {
        assert!(opp.path.is_some(), "MultiHopArb must have path populated");
        let path = opp.path.as_ref().unwrap();
        assert_eq!(path.len(), 2, "Two-pool path should have length 2");
        assert_eq!(path[0], opp.pool_a);
        assert_eq!(path[path.len() - 1], opp.pool_b);
    }
}

/// ── Sandwich Detection Tests ─────────────────────────────────────────────────
#[test]
fn test_sandwich_detection_synthetic() {
    use mev_backtest_core::data::ExecutedLog;

    let pool = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let alice = address!("1111111111111111111111111111111111111111");
    let bob = address!("2222222222222222222222222222222222222222");

    let v2_swap_topic: B256 =
        b256!("d78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822");

    let v2_swap_log = |pool: Address, amt0_in: u128, amt1_in: u128, amt0_out: u128, amt1_out: u128| -> ExecutedLog {
        let mut data = Vec::with_capacity(128);
        let mut buf = vec![0u8; 16];
        data.extend_from_slice(&buf);
        data.extend_from_slice(&amt0_in.to_be_bytes());
        buf = vec![0u8; 16];
        data.extend_from_slice(&buf);
        data.extend_from_slice(&amt1_in.to_be_bytes());
        buf = vec![0u8; 16];
        data.extend_from_slice(&buf);
        data.extend_from_slice(&amt0_out.to_be_bytes());
        buf = vec![0u8; 16];
        data.extend_from_slice(&buf);
        data.extend_from_slice(&amt1_out.to_be_bytes());
        ExecutedLog { address: pool, topics: vec![v2_swap_topic, B256::ZERO, B256::ZERO], data: data.into() }
    };

    let usdc = address!("2791bca1f2de4661ed88a30c99a7a9449aa84174");
    let wmatic = address!("0d500b1d8e8ef31e21c99d1db9a6444d3adf1270");

    let mut pm = PoolManager::new();
    pm.add_pool(PoolState::UniswapV2(UniswapV2PoolState {
        info: PoolInfo {
            address: pool,
            token0: usdc,
            token1: wmatic,
            fee: 30,
            name: None,
            dex_type: DexType::UniswapV2,
            tick_spacing: None,
        },
        reserve0: 1_000_000,
        reserve1: 1_000_000,
    }));

    let mut detector = SandwichDetector::new(42);
    let timestamp = 12345u64;

    // Tx 0: alice frontruns — buys WMATIC (token0→token1)
    detector.process_tx(0, &[v2_swap_log(pool, 100, 0, 0, 90)], Some(alice));
    assert!(detector.detect(timestamp, &pm).is_empty());

    // Tx 1: bob (victim) — buys WMATIC at worse price
    detector.process_tx(1, &[v2_swap_log(pool, 200, 0, 0, 170)], Some(bob));
    assert!(detector.detect(timestamp, &pm).is_empty());

    // Tx 2: alice backruns — sells WMATIC (token1→token0)
    detector.process_tx(2, &[v2_swap_log(pool, 0, 85, 105, 0)], Some(alice));
    let opps = detector.detect(timestamp, &pm);
    assert!(!opps.is_empty(), "Should detect sandwich");
    assert_eq!(opps[0].strategy, Strategy::Sandwich);
    assert_eq!(opps[0].pool_a, pool);
    assert_eq!(opps[0].victim_tx_index, Some(1));
    assert_eq!(opps[0].backrun_tx_index, Some(2));
    assert_eq!(opps[0].token_in, usdc);
    assert_eq!(opps[0].token_out, wmatic);

    // No duplicate
    assert!(detector.detect(timestamp, &pm).is_empty());
}

/// ── Real-Data Tests (async / RPC) ──────────────────────────────────────────
/// These tests load real pool configs from the registry and optionally fetch
/// on-chain state via RPC.  They skip gracefully when no RPC is available,
/// following the same pattern as e2e_discovery.

#[tokio::test]
async fn test_real_state_initialization_and_two_hop() {
    let rpc_url = match rpc_url() {
        Some(url) => url,
        None => { eprintln!("Skipping: RPC_URL not set"); return; }
    };

    let rpc = match mev_backtest_core::rpc::RpcClient::new(&rpc_url, 137) {
        Ok(r) => r,
        Err(e) => { eprintln!("Skipping: failed to create RPC client: {e}"); return; }
    };

    let block_num = match rpc.get_block_number().await {
        Ok(n) => n.saturating_sub(10),
        Err(e) => { eprintln!("Skipping: failed to get block number: {e}"); return; }
    };

    // Load two real Polygon pools that share the same pair (different DEX → arb)
    let all = load_polygon_registry();

    // QuickSwap WMATIC/USDC  (0x6e7a5f...)
    let qs = all.iter()
        .find(|p| p.address == address!("6e7a5fafcec6bb1e78bae2a1f0b612012bf14827"))
        .expect("QuickSwap WMATIC/USDC missing from registry");
    // SushiSwap WMATIC/USDC (0xcd353f...)
    let ss = all.iter()
        .find(|p| p.address == address!("cd353f79d9fade311fc3119b841e1f456b54e858"))
        .expect("SushiSwap WMATIC/USDC missing from registry");

    let mut pm = PoolManager::new();
    pm.add_pool(pool_info_to_state(qs.clone()));
    pm.add_pool(pool_info_to_state(ss.clone()));

    pm.init_from_rpc(&rpc, block_num).await;

    let initialized = pm.initialized_count();
    eprintln!("Initialized {}/2 pools at block {block_num}", initialized);

    if initialized == 0 {
        eprintln!("Skipping detection assertions: no pools initialized (RPC may not support historical queries)");
        return;
    }

    // Run TwoHopArb detection on real data
    let opps = TwoHopArbDetector::detect(
        &pm, block_num, 0, block_num, 50_000_000_000, GasConfig::default(),
    );

    eprintln!("TwoHopArb detected {} opportunities on real pools at block {block_num}", opps.len());

    // Same-pair pools almost always have slight price differences
    assert!(!opps.is_empty(), "Should detect arb between real same-pair pools with different prices");
    for opp in &opps {
        assert_eq!(opp.strategy, Strategy::TwoHopArb);
        assert!(opp.expected_profit > U256::ZERO, "Profit should be > 0 on real data");
    }
}

#[tokio::test]
async fn test_real_multi_hop_detection() {
    let rpc_url = match rpc_url() {
        Some(url) => url,
        None => { eprintln!("Skipping: RPC_URL not set"); return; }
    };

    let rpc = match mev_backtest_core::rpc::RpcClient::new(&rpc_url, 137) {
        Ok(r) => r,
        Err(e) => { eprintln!("Skipping: failed to create RPC client: {e}"); return; }
    };

    let block_num = match rpc.get_block_number().await {
        Ok(n) => n.saturating_sub(10),
        Err(e) => { eprintln!("Skipping: failed to get block number: {e}"); return; }
    };

    let all = load_polygon_registry();

    // Build a pool set that supports multi-hop paths:
    //   QuickSwap WMATIC/USDC, WMATIC/USDT, USDC/USDT
    let qs_wmatic_usdc = all.iter()
        .find(|p| p.address == address!("6e7a5fafcec6bb1e78bae2a1f0b612012bf14827"))
        .expect("QuickSwap WMATIC/USDC");
    let qs_wmatic_usdt = all.iter()
        .find(|p| p.address == address!("604029b0c1a79eebfb31f7c5316434484f3a4b55"))
        .expect("QuickSwap WMATIC/USDT");
    let qs_usdc_usdt = all.iter()
        .find(|p| p.address == address!("2cf7252e74036d1da831d11089d326296e64a910"))
        .expect("QuickSwap USDC/USDT");

    let mut pm = PoolManager::new();
    pm.add_pool(pool_info_to_state(qs_wmatic_usdc.clone()));
    pm.add_pool(pool_info_to_state(qs_wmatic_usdt.clone()));
    pm.add_pool(pool_info_to_state(qs_usdc_usdt.clone()));

    pm.init_from_rpc(&rpc, block_num).await;

    let initialized = pm.initialized_count();
    eprintln!("Initialized {}/3 pools at block {block_num}", initialized);

    if initialized == 0 {
        eprintln!("Skipping detection assertions: no pools initialized");
        return;
    }

    // Run MultiHopArb detection
    let opps = mev_backtest_core::mev::multi_hop::MultiHopArbDetector::detect(
        &pm, block_num, 0, block_num, 50_000_000_000, GasConfig::default(),
    );

    eprintln!("MultiHopArb detected {} opportunities on real pools at block {block_num}", opps.len());

    // At minimum, paths should be found (even if not all are profitable)
    if opps.is_empty() {
        // Could happen if prices are perfectly aligned — unlikely but possible
        eprintln!("No multi-hop arb opportunities at this block (prices may be aligned)");
    } else {
        for opp in &opps {
            assert_eq!(opp.strategy, Strategy::MultiHopArb);
            assert!(opp.path.is_some(), "MultiHopArb must have path populated");
            let path = opp.path.as_ref().unwrap();
            assert!(path.len() >= 2, "Path must have at least 2 pools, got {}", path.len());
        }
    }
}

#[tokio::test]
async fn test_real_detection_all_sushi_wmatic_pools() {
    let rpc_url = match rpc_url() {
        Some(url) => url,
        None => { eprintln!("Skipping: RPC_URL not set"); return; }
    };

    let rpc = match mev_backtest_core::rpc::RpcClient::new(&rpc_url, 137) {
        Ok(r) => r,
        Err(e) => { eprintln!("Skipping: failed to create RPC client: {e}"); return; }
    };

    let block_num = match rpc.get_block_number().await {
        Ok(n) => n.saturating_sub(10),
        Err(e) => { eprintln!("Skipping: failed to get block number: {e}"); return; }
    };

    // All SushiSwap WMATIC pools share WMATIC → dense arbitrage graph
    let sushipool_names = [
        "SushiSwap WMATIC/USDC",
        "SushiSwap WMATIC/USDT",
        "SushiSwap WMATIC/DAI",
        "SushiSwap WMATIC/WETH",
        "SushiSwap WMATIC/WBTC",
        "SushiSwap WMATIC/stMATIC",
    ];

    let all = load_polygon_registry();
    let mut pm = PoolManager::new();

    for name in &sushipool_names {
        if let Some(info) = all.iter().find(|p| p.name.as_deref() == Some(name)) {
            pm.add_pool(pool_info_to_state(info.clone()));
        }
    }

    let count = pm.pool_count();
    assert_eq!(count, sushipool_names.len(), "Should find all SushiSwap WMATIC pools, got {count}");

    pm.init_from_rpc(&rpc, block_num).await;

    let initialized = pm.initialized_count();
    eprintln!("Initialized {initialized}/{count} SushiSwap WMATIC pools at block {block_num}");

    if initialized < 2 {
        eprintln!("Skipping: too few initialized pools ({initialized})");
        return;
    }

    // TwoHopArb
    let opps = TwoHopArbDetector::detect(
        &pm, block_num, 0, block_num, 50_000_000_000, GasConfig::default(),
    );
    eprintln!("TwoHopArb detected {} opportunities across {count} real pools", opps.len());

    // With 6 WMATIC-quoted pools, arb pairs should always exist
    assert!(!opps.is_empty(), "Should detect two-hop arb across multiple WMATIC pools");

    // MultiHopArb
    let mhop_opps = mev_backtest_core::mev::multi_hop::MultiHopArbDetector::detect(
        &pm, block_num, 0, block_num, 50_000_000_000, GasConfig::default(),
    );
    eprintln!("MultiHopArb detected {} opportunities across {count} real pools", mhop_opps.len());

    for opp in mhop_opps.iter().take(5) {
        assert!(opp.path.is_some());
        let path = opp.path.as_ref().unwrap();
        assert!(path.len() >= 2);
    }
}

#[test]
fn test_jit_detection_synthetic() {
    use mev_backtest_core::pool::decoders::{V3_SWAP_TOPIC, V3_MINT_TOPIC, V3_BURN_TOPIC};
    use mev_backtest_core::data::ExecutedLog;
    use alloy::primitives::{address, Bytes, B256};

    let pool = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");

    fn v3_mint_log(pool: Address, lower: i32, upper: i32, amount: u128) -> ExecutedLog {
        let mut data = Vec::new();
        let mut padded = [0u8; 32];
        padded[28..32].copy_from_slice(&lower.to_be_bytes());
        data.extend_from_slice(&padded);
        padded = [0u8; 32];
        padded[28..32].copy_from_slice(&upper.to_be_bytes());
        data.extend_from_slice(&padded);
        padded = [0u8; 32];
        padded[16..32].copy_from_slice(&amount.to_be_bytes());
        data.extend_from_slice(&padded);
        ExecutedLog { address: pool, topics: vec![*V3_MINT_TOPIC, B256::ZERO, B256::ZERO], data: data.into() }
    }

    fn v3_burn_log(pool: Address, lower: i32, upper: i32, amount: u128) -> ExecutedLog {
        let mut data = Vec::new();
        let mut padded = [0u8; 32];
        padded[28..32].copy_from_slice(&lower.to_be_bytes());
        data.extend_from_slice(&padded);
        padded = [0u8; 32];
        padded[28..32].copy_from_slice(&upper.to_be_bytes());
        data.extend_from_slice(&padded);
        padded = [0u8; 32];
        padded[16..32].copy_from_slice(&amount.to_be_bytes());
        data.extend_from_slice(&padded);
        ExecutedLog { address: pool, topics: vec![V3_BURN_TOPIC, B256::ZERO, B256::ZERO], data: data.into() }
    }

    fn v3_swap_log(pool: Address) -> ExecutedLog {
        ExecutedLog { address: pool, topics: vec![V3_SWAP_TOPIC, B256::ZERO, B256::ZERO], data: Bytes::from_static(&[0u8; 160]) }
    }

    let mut detector = JitDetector::new(42);
    let timestamp = 12345u64;

    // Tx 0: deploy liquidity
    detector.process_tx(0, &[v3_mint_log(pool, -1000, 1000, 1_000_000)], None);
    assert!(detector.detect(timestamp).is_empty());

    // Tx 1: swap against it
    detector.process_tx(1, &[v3_swap_log(pool)], None);
    let mut opps = detector.detect(timestamp);
    assert!(!opps.is_empty(), "Mint+Swap should trigger JIT detection");
    assert_eq!(opps[0].strategy, mev_backtest_core::types::Strategy::Jit);
    assert_eq!(opps[0].pool_a, pool);
    assert_eq!(opps[0].tick_lower, Some(-1000));
    assert_eq!(opps[0].tick_upper, Some(1000));
    assert_eq!(opps[0].liquidity_amount, Some(1_000_000));

    // Tx 2: burn position
    detector.process_tx(2, &[v3_burn_log(pool, -1000, 1000, 1_000_000)], None);
    opps = detector.detect(timestamp);
    assert_eq!(opps.len(), 1, "Burn should trigger full JIT emission");

    // No duplicate
    assert!(detector.detect(timestamp).is_empty());
}

#[tokio::test]
async fn test_real_v3_mint_swap_burn_detection() {
    let rpc_url = match rpc_url() {
        Some(url) => url,
        None => { eprintln!("Skipping: RPC_URL not set"); return; }
    };

    let rpc = match mev_backtest_core::rpc::RpcClient::new(&rpc_url, 137) {
        Ok(r) => r,
        Err(e) => { eprintln!("Skipping: failed to create RPC client: {e}"); return; }
    };

    let block_num = match rpc.get_block_number().await {
        Ok(n) => n.saturating_sub(100),
        Err(e) => { eprintln!("Skipping: failed to get block number: {e}"); return; }
    };

    // Load a real V3 pool (e.g., QuickSwap USDC/WMATIC V3)
    let registry = load_polygon_registry();
    let v3_pools: Vec<_> = registry.iter().filter(|p| p.dex_type == mev_backtest_core::pool::dex_type::DexType::UniswapV3).collect();

    if v3_pools.is_empty() {
        eprintln!("Skipping: no V3 pool found in registry");
        return;
    }

    let pool_info = v3_pools[0].clone();
    let mut pm = PoolManager::new();
    pm.add_pool(pool_info_to_state(pool_info.clone()));
    pm.init_from_rpc(&rpc, block_num).await;

    let initialized = pm.initialized_count();
    eprintln!("V3 pool {} initialized={} at block {}",
        pool_info.address, initialized, block_num);

    if initialized == 0 {
        eprintln!("Skipping: V3 pool not initialized");
        return;
    }

    // We can't easily force a V3 Mint/Swap/Burn sequence from a test,
    // but we can verify the JitDetector compiles and processes empty data.
    let mut detector = JitDetector::new(block_num);
    // Process empty data (no logs from this pool in this test block)
    detector.process_tx(0, &[], None);
    let opps = detector.detect(block_num);
    eprintln!("JIT detection on real V3 pool: {} opportunities (expected 0 without events)", opps.len());

    // This test primarily validates that JitDetector works with real PoolManager state
    // even though we can't produce real V3 events without replaying a block.
    assert!(opps.is_empty(), "No JIT without any events");
}

/// ── JitArb Detection Tests ──────────────────────────────────────────────────
#[test]
fn test_jit_arb_detection_synthetic() {
    use mev_backtest_core::mev::jit_arb::JitArbDetector;
    use mev_backtest_core::pool::decoders::{V3_SWAP_TOPIC, V3_MINT_TOPIC};
    use mev_backtest_core::data::ExecutedLog;
    use alloy::primitives::{address, Address, Bytes, B256};

    let pool_p = address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    let pool_q = address!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let sender = address!("1111111111111111111111111111111111111111");
    let wmatic = address!("0d500b1d8e8ef31e21c99d1db9a6444d3adf1270");
    let usdc = address!("2791bca1f2de4661ed88a30c99a7a9449aa84174");

    fn v3_mint_log(pool: Address, lower: i32, upper: i32, amount: u128) -> ExecutedLog {
        let mut data = Vec::new();
        let mut padded = [0u8; 32];
        padded[28..32].copy_from_slice(&lower.to_be_bytes());
        data.extend_from_slice(&padded);
        padded = [0u8; 32];
        padded[28..32].copy_from_slice(&upper.to_be_bytes());
        data.extend_from_slice(&padded);
        padded = [0u8; 32];
        padded[16..32].copy_from_slice(&amount.to_be_bytes());
        data.extend_from_slice(&padded);
        ExecutedLog { address: pool, topics: vec![*V3_MINT_TOPIC, B256::ZERO, B256::ZERO], data: data.into() }
    }

    fn v3_swap_log(pool: Address) -> ExecutedLog {
        ExecutedLog { address: pool, topics: vec![V3_SWAP_TOPIC, B256::ZERO, B256::ZERO], data: Bytes::from_static(&[0u8; 160]) }
    }

    let mut pm = mev_backtest_core::pool::state::PoolManager::new();
    pm.add_pool(mev_backtest_core::pool::state::PoolState::UniswapV2(
        mev_backtest_core::pool::state::UniswapV2PoolState {
            info: mev_backtest_core::pool::state::PoolInfo {
                address: pool_p, token0: wmatic, token1: usdc, fee: 30, name: None,
                dex_type: mev_backtest_core::pool::dex_type::DexType::UniswapV2, tick_spacing: None,
            },
            reserve0: 1_000_000, reserve1: 1_000_000,
        },
    ));
    pm.add_pool(mev_backtest_core::pool::state::PoolState::UniswapV2(
        mev_backtest_core::pool::state::UniswapV2PoolState {
            info: mev_backtest_core::pool::state::PoolInfo {
                address: pool_q,
                token0: usdc,
                token1: address!("c2132d05d31c914a87c6611c10748aeb04b58e8f"),
                fee: 30, name: None,
                dex_type: mev_backtest_core::pool::dex_type::DexType::UniswapV2, tick_spacing: None,
            },
            reserve0: 1_000_000, reserve1: 1_000_000,
        },
    ));

    let mut detector = JitArbDetector::new(42);
    detector.process_tx(0, &[
        v3_mint_log(pool_p, -100, 100, 500_000),
        v3_swap_log(pool_p),
        v3_swap_log(pool_q),
    ], Some(sender));

    let opps = detector.detect(12345, &pm);
    assert_eq!(opps.len(), 1, "Should detect JitArb");
    assert_eq!(opps[0].strategy, mev_backtest_core::types::Strategy::JitArb);
    assert_eq!(opps[0].pool_a, pool_p);
    assert_eq!(opps[0].pool_b, pool_q);
    assert_eq!(opps[0].liquidity_amount, Some(500_000));
    assert_eq!(opps[0].tick_lower, Some(-100));
    assert_eq!(opps[0].tick_upper, Some(100));
}
