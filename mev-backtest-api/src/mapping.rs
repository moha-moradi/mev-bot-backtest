use alloy::primitives::Address;
use mev_backtest_core::mev::opportunity::MevOpportunity;
use mev_backtest_core::types::Strategy;

use crate::state::{SimulationTrace, TraceResult, TraceStep, UiOpportunity};

const WEI_PER_ETH: f64 = 1_000_000_000_000_000_000.0;
const BUILDER_TIP_PCT: f64 = 0.10;
const FLASH_LOAN_FEE_PCT: f64 = 0.0009;
const MIN_PROFIT_THRESHOLD: f64 = 0.001;

fn wei_to_eth(wei: u128) -> f64 {
    wei as f64 / WEI_PER_ETH
}

fn ui_strategy(strategy: Strategy) -> &'static str {
    match strategy {
        Strategy::TwoHopArb | Strategy::MultiHopArb => "arb",
        Strategy::Jit => "jit",
        Strategy::JitArb => "jitarb",
        Strategy::Sandwich => "sandwich",
    }
}

fn short_hash(addr: &Address) -> String {
    let s = hex::encode(addr.as_slice());
    format!("0x{}...{}", &s[..4], &s[s.len() - 4..])
}

fn build_arb_trace(
    opp: &MevOpportunity,
    gross: f64,
    gas: f64,
    net: f64,
    token_pair: Option<&str>,
    dex_path: &[String],
    flash_provider: Option<&str>,
    flash_size: Option<f64>,
) -> SimulationTrace {
    let mut steps = vec![
        TraceStep {
            label: "Block".to_string(),
            value: Some(format!("#{}", opp.block_number)),
            sub: None,
        },
    ];
    if let Some(tp) = token_pair {
        steps.push(TraceStep {
            label: "Token pair".to_string(),
            value: Some(tp.to_string()),
            sub: None,
        });
    }
    if !dex_path.is_empty() {
        steps.push(TraceStep {
            label: "Path".to_string(),
            value: Some(dex_path.join(" → ")),
            sub: None,
        });
    }
    if let (Some(provider), Some(size)) = (flash_provider, flash_size) {
        steps.push(TraceStep {
            label: "Flash loan".to_string(),
            value: Some(format!("{size} via {provider}")),
            sub: None,
        });
    }
    steps.push(TraceStep {
        label: "Gross revenue".to_string(),
        value: Some(format!("{gross:.5}")),
        sub: None,
    });
    steps.push(TraceStep {
        label: "Gas".to_string(),
        value: Some(format!("−{gas:.5}")),
        sub: None,
    });
    SimulationTrace {
        title: "Arbitrage trace".to_string(),
        steps,
        result: TraceResult { gross, cost: gas, net },
    }
}

fn build_jit_trace(opp: &MevOpportunity, gross: f64, gas: f64, net: f64) -> SimulationTrace {
    SimulationTrace {
        title: "JIT Liquidity trace".to_string(),
        steps: vec![
            TraceStep { label: "Block".to_string(), value: Some(format!("#{}", opp.block_number)), sub: None },
            TraceStep { label: "Target pool".to_string(), value: opp.path.as_ref().and_then(|p| p.first()).map(short_hash), sub: None },
            TraceStep { label: "Incoming swap detected".to_string(), value: None, sub: None },
            TraceStep { label: "Mint LP".to_string(), value: opp.liquidity_amount.map(|a| format!("{a}")), sub: None },
            TraceStep { label: "Burn LP".to_string(), value: None, sub: None },
            TraceStep { label: "Fees earned".to_string(), value: Some(format!("{gross:.5}")), sub: None },
            TraceStep { label: "Gas".to_string(), value: Some(format!("−{gas:.5}")), sub: None },
        ],
        result: TraceResult { gross, cost: gas, net },
    }
}

fn build_sandwich_trace(opp: &MevOpportunity, gross: f64, gas: f64, net: f64) -> SimulationTrace {
    SimulationTrace {
        title: "Sandwich trace".to_string(),
        steps: vec![
            TraceStep { label: "Block".to_string(), value: Some(format!("#{}", opp.block_number)), sub: None },
            TraceStep { label: "Victim tx".to_string(), value: opp.victim_tx_index.map(|i| format!("tx #{i}")), sub: None },
            TraceStep { label: "Front-run".to_string(), value: None, sub: None },
            TraceStep { label: "Victim executes".to_string(), value: None, sub: None },
            TraceStep { label: "Back-run".to_string(), value: None, sub: None },
            TraceStep { label: "Gross capture".to_string(), value: Some(format!("{gross:.5}")), sub: None },
            TraceStep { label: "Gas".to_string(), value: Some(format!("−{gas:.5}")), sub: None },
        ],
        result: TraceResult { gross, cost: gas, net },
    }
}

fn build_jitarb_trace(opp: &MevOpportunity, gross: f64, gas: f64, net: f64) -> SimulationTrace {
    SimulationTrace {
        title: "JIT+Arb trace".to_string(),
        steps: vec![
            TraceStep { label: "Block".to_string(), value: Some(format!("#{}", opp.block_number)), sub: None },
            TraceStep { label: "Flash loan".to_string(), value: None, sub: None },
            TraceStep { label: "JIT mint".to_string(), value: opp.liquidity_amount.map(|a| format!("{a}")), sub: None },
            TraceStep { label: "Victim swap".to_string(), value: None, sub: None },
            TraceStep { label: "Burn LP / arb exit".to_string(), value: None, sub: None },
            TraceStep { label: "Repay".to_string(), value: None, sub: None },
            TraceStep { label: "Gross + FL fee".to_string(), value: Some(format!("{gross:.5}")), sub: None },
            TraceStep { label: "Gas".to_string(), value: Some(format!("−{gas:.5}")), sub: None },
        ],
        result: TraceResult { gross, cost: gas, net },
    }
}

pub fn map_opportunity(
    opp: &MevOpportunity,
    _pool_registry: &mev_backtest_core::pool::registry::PoolRegistry,
    is_flash_loan: bool,
    block_hash: &str,
) -> UiOpportunity {
    let strategy = ui_strategy(opp.strategy);
    let gross = wei_to_eth(opp.expected_profit.to::<u128>());
    let gas = wei_to_eth(opp.gas_cost_wei);
    let flash_fee = if is_flash_loan { gross * FLASH_LOAN_FEE_PCT } else { 0.0 };
    let builder_tip = gross * BUILDER_TIP_PCT;
    let net = gross - gas - flash_fee - builder_tip;

    let result = if net > MIN_PROFIT_THRESHOLD {
        "profitable"
    } else if net > 0.0 {
        "below_threshold"
    } else {
        "reverted"
    };

    let explorer_url = format!("https://etherscan.io/tx/0x{}", block_hash);

    let token_pair = Some(format!("{:?}/{:?}", opp.token_in, opp.token_out));
    let dex_path: Vec<String> = vec![short_hash(&opp.pool_a), short_hash(&opp.pool_b)];

    let simulation_trace = match strategy {
        "arb" => build_arb_trace(opp, gross, gas, net, token_pair.as_deref(), &dex_path, if is_flash_loan { Some("Balancer v2") } else { None }, if is_flash_loan { Some(gross) } else { None }),
        "jit" => build_jit_trace(opp, gross, gas, net),
        "jitarb" => build_jitarb_trace(opp, gross, gas, net),
        "sandwich" => build_sandwich_trace(opp, gross, gas, net),
        _ => build_arb_trace(opp, gross, gas, net, token_pair.as_deref(), &dex_path, None, None),
    };

    UiOpportunity {
        id: format!("{}-{}-{}", strategy, opp.block_number, opp.tx_index),
        tx_hash: format!("0x{}", block_hash),
        block_number: opp.block_number,
        timestamp: opp.timestamp,
        strategy: strategy.to_string(),
        gross_revenue: gross,
        gas_cost: gas,
        flash_loan_fee: flash_fee,
        builder_tip,
        net_profit: net,
        result: result.to_string(),
        explorer_url,
        token_pair,
        dex_path: Some(dex_path),
        pool_a: Some(format!("{:?}", opp.pool_a)),
        pool_b: Some(format!("{:?}", opp.pool_b)),
        input_amount: Some(opp.input_amount.to_string()),
        flash_loan_provider: if is_flash_loan { Some("Balancer v2".to_string()) } else { None },
        flash_loan_size: if is_flash_loan { Some(gross) } else { None },
        victim_tx_hash: opp.victim_tx_index.map(|i| format!("tx #{i}")),
        front_run_size: Some(0.0),
        victim_slippage: Some(0.0),
        gross_capture: Some(gross),
        simulation_trace,
    }
}

pub fn map_opportunities(
    opportunities: &[MevOpportunity],
) -> Vec<UiOpportunity> {
    let registry = mev_backtest_core::pool::registry::PoolRegistry;
    opportunities
        .iter()
        .map(|opp| {
            let is_fl = matches!(opp.strategy, Strategy::JitArb | Strategy::MultiHopArb);
            map_opportunity(opp, &registry, is_fl, &format!("{:x}", opp.block_number))
        })
        .collect()
}
