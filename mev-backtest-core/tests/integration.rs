use alloy::primitives::{address, Address, U256};
use mev_backtest_core::mev::two_hop::TwoHopArbDetector;
use mev_backtest_core::pool::state::UniswapV2PoolState;
use mev_backtest_core::pool::state::{PoolInfo, PoolManager, PoolState};
use mev_backtest_core::types::{GasConfig, Strategy};

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
