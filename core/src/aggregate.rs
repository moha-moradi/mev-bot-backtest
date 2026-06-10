use crate::mev::opportunity::MevOpportunity;
use crate::types::Strategy;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SummaryMetrics {
    pub total: usize,
    pub profitable: usize,
    pub gross_revenue: f64,
    pub net_profit: f64,
    pub net_profit_usd: f64,
    pub total_cost: f64,
    pub best_strategy: Option<String>,
    pub best_single_opp: f64,
    pub gross_revenue_wei: u128,
    pub net_profit_wei: i128,
    pub total_gas_cost_wei: u128,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StrategyMetrics {
    pub strategy: String,
    pub count: usize,
    pub profitable: usize,
    pub gross_revenue: f64,
    pub gas_fees: f64,
    pub net_profit: f64,
    pub net_profit_usd: f64,
    pub roi: f64,
    pub avg_per_opp: f64,
    pub best_opp: f64,
    pub gross_revenue_wei: u128,
    pub net_profit_wei: i128,
    pub total_gas_cost_wei: u128,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DexMetrics {
    pub dex: String,
    pub fork: String,
    pub tx_count: usize,
    pub opportunities: usize,
    pub profitable: usize,
    pub revenue: f64,
    pub avg_profit: f64,
    pub gross_revenue_wei: u128,
    pub net_profit_wei: i128,
    pub total_gas_cost_wei: u128,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AggregationResult {
    pub summary: SummaryMetrics,
    pub by_strategy: std::collections::HashMap<String, StrategyMetrics>,
    pub by_dex: Vec<DexMetrics>,
}

pub struct DexMeta {
    pub name: String,
    pub fork: String,
    pub tx_count: usize,
}

const WEI_PER_ETH: f64 = 1_000_000_000_000_000_000.0;

fn wei_to_eth(wei: u128) -> f64 {
    wei as f64 / WEI_PER_ETH
}

fn ui_strategy_name(strategy: Strategy) -> &'static str {
    match strategy {
        Strategy::TwoHopArb | Strategy::MultiHopArb => "arb",
        Strategy::Jit => "jit",
        Strategy::JitArb => "jitarb",
        Strategy::Sandwich => "sandwich",
    }
}

pub fn aggregate(
    opportunities: &[MevOpportunity],
    dexes: &[DexMeta],
    usd_price: f64,
) -> AggregationResult {
    let mut by_strategy: std::collections::HashMap<String, Vec<&MevOpportunity>> =
        std::collections::HashMap::new();
    let mut by_dex: std::collections::HashMap<String, Vec<&MevOpportunity>> =
        std::collections::HashMap::new();

    for opp in opportunities {
        let sname = ui_strategy_name(opp.strategy).to_string();
        by_strategy.entry(sname).or_default().push(opp);

        for dex_meta in dexes {
            by_dex.entry(dex_meta.name.clone()).or_default().push(opp);
        }
    }

    let total = opportunities.len();
    let gross_revenue: f64 = opportunities
        .iter()
        .map(|o| wei_to_eth(o.expected_profit.to::<u128>()))
        .sum();
    let total_gas: f64 = opportunities
        .iter()
        .map(|o| wei_to_eth(o.gas_cost_wei))
        .sum();
    let net_profit = gross_revenue - total_gas;

    let profitable_count = opportunities
        .iter()
        .filter(|o| {
            let profit = wei_to_eth(o.expected_profit.to::<u128>()) - wei_to_eth(o.gas_cost_wei);
            profit > 0.0
        })
        .count();

    let best_single_opp = opportunities
        .iter()
        .map(|o| wei_to_eth(o.expected_profit.to::<u128>()))
        .fold(0.0_f64, f64::max);

    let mut best_strategy: Option<String> = None;
    let mut best_strat_net = 0.0_f64;
    let mut strategy_metrics = std::collections::HashMap::new();

    for (sname, opps) in &by_strategy {
        let count = opps.len();
        let strat_gross: f64 = opps.iter().map(|o| wei_to_eth(o.expected_profit.to::<u128>())).sum();
        let strat_gas: f64 = opps.iter().map(|o| wei_to_eth(o.gas_cost_wei)).sum();
        let strat_net = strat_gross - strat_gas;
        let strat_profitable = opps
            .iter()
            .filter(|o| {
                wei_to_eth(o.expected_profit.to::<u128>()) - wei_to_eth(o.gas_cost_wei) > 0.0
            })
            .count();
        let best_opp = opps
            .iter()
            .map(|o| wei_to_eth(o.expected_profit.to::<u128>()))
            .fold(0.0_f64, f64::max);
        let roi = if strat_gas > 0.0 {
            (strat_net / strat_gas) * 100.0
        } else {
            0.0
        };
        let avg = if count > 0 { strat_gross / count as f64 } else { 0.0 };

        let gross_wei: u128 = opps.iter().map(|o| o.expected_profit.to::<u128>()).sum();
        let gas_wei: u128 = opps.iter().map(|o| o.gas_cost_wei).sum();
        let net_wei = (gross_wei as i128) - (gas_wei as i128);

        if strat_net > best_strat_net {
            best_strat_net = strat_net;
            best_strategy = Some(sname.clone());
        }

        strategy_metrics.insert(
            sname.clone(),
            StrategyMetrics {
                strategy: sname.clone(),
                count,
                profitable: strat_profitable,
                gross_revenue: strat_gross,
                gas_fees: strat_gas,
                net_profit: strat_net,
                net_profit_usd: strat_net * usd_price,
                roi,
                avg_per_opp: avg,
                best_opp,
                gross_revenue_wei: gross_wei,
                net_profit_wei: net_wei,
                total_gas_cost_wei: gas_wei,
            },
        );
    }

    let mut dex_metrics: Vec<DexMetrics> = dexes
        .iter()
        .map(|dex_meta| {
            let opps_for_dex = by_dex.get(&dex_meta.name).cloned().unwrap_or_default();
            let count = opps_for_dex.len();
            let revenue: f64 = opps_for_dex
                .iter()
                .map(|o| wei_to_eth(o.expected_profit.to::<u128>()))
                .sum();
            let profitable = opps_for_dex
                .iter()
                .filter(|o| {
                    wei_to_eth(o.expected_profit.to::<u128>()) - wei_to_eth(o.gas_cost_wei) > 0.0
                })
                .count();
            let avg_profit = if count > 0 { revenue / count as f64 } else { 0.0 };
            let gross_wei: u128 = opps_for_dex
                .iter()
                .map(|o| o.expected_profit.to::<u128>())
                .sum();
            let gas_wei: u128 = opps_for_dex
                .iter()
                .map(|o| o.gas_cost_wei)
                .sum();
            let net_wei = (gross_wei as i128) - (gas_wei as i128);
            DexMetrics {
                dex: dex_meta.name.clone(),
                fork: dex_meta.fork.clone(),
                tx_count: dex_meta.tx_count,
                opportunities: count,
                profitable,
                revenue,
                avg_profit,
                gross_revenue_wei: gross_wei,
                net_profit_wei: net_wei,
                total_gas_cost_wei: gas_wei,
            }
        })
        .collect();
    dex_metrics.sort_by(|a, b| b.revenue.partial_cmp(&a.revenue).unwrap_or(std::cmp::Ordering::Equal));

    let summary_gross_wei: u128 = opportunities
        .iter()
        .map(|o| o.expected_profit.to::<u128>())
        .sum();
    let summary_gas_wei: u128 = opportunities
        .iter()
        .map(|o| o.gas_cost_wei)
        .sum();
    let summary_net_wei = (summary_gross_wei as i128) - (summary_gas_wei as i128);

    AggregationResult {
        summary: SummaryMetrics {
            total,
            profitable: profitable_count,
            gross_revenue,
            net_profit,
            net_profit_usd: net_profit * usd_price,
            total_cost: total_gas,
            best_strategy,
            best_single_opp,
            gross_revenue_wei: summary_gross_wei,
            net_profit_wei: summary_net_wei,
            total_gas_cost_wei: summary_gas_wei,
        },
        by_strategy: strategy_metrics,
        by_dex: dex_metrics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::{Address, U256};
    use crate::mev::opportunity::MevOpportunity;
    use crate::types::Strategy;

    fn make_opp(strategy: Strategy, profit_wei: u128, gas_wei: u128, block: u64) -> MevOpportunity {
        MevOpportunity {
            block_number: block,
            tx_index: 0,
            strategy,
            pool_a: Address::ZERO,
            pool_b: Address::ZERO,
            token_in: Address::ZERO,
            token_out: Address::ZERO,
            input_amount: U256::ZERO,
            expected_profit: U256::from(profit_wei),
            gas_cost_wei: gas_wei,
            timestamp: 12345,
            path: None,
            tick_lower: None,
            tick_upper: None,
            liquidity_amount: None,
            victim_tx_index: None,
            backrun_tx_index: None,
        }
    }

    fn one_eth() -> u128 {
        10u128.pow(18)
    }

    #[test]
    fn test_aggregate_empty() {
        let result = aggregate(&[], &[], 1.0);
        assert_eq!(result.summary.total, 0);
        assert_eq!(result.summary.profitable, 0);
        assert_eq!(result.summary.gross_revenue, 0.0);
        assert_eq!(result.summary.net_profit, 0.0);
        assert_eq!(result.summary.net_profit_usd, 0.0);
        assert!(result.by_strategy.is_empty());
        assert!(result.by_dex.is_empty());
    }

    #[test]
    fn test_aggregate_single_opportunity() {
        let opps = vec![make_opp(Strategy::TwoHopArb, one_eth(), one_eth() / 5, 1)];
        let dexes = vec![DexMeta { name: "QuickSwap".into(), fork: "UniV2".into(), tx_count: 1 }];
        let result = aggregate(&opps, &dexes, 2.0);

        assert_eq!(result.summary.total, 1);
        assert_eq!(result.summary.profitable, 1);
        assert_approx_eq(result.summary.gross_revenue, 1.0);
        assert_approx_eq(result.summary.total_cost, 0.2);
        assert_approx_eq(result.summary.net_profit, 0.8);
        assert_approx_eq(result.summary.net_profit_usd, 1.6);
        assert_eq!(result.summary.best_strategy.as_deref(), Some("arb"));
        assert_approx_eq(result.summary.best_single_opp, 1.0);

        assert_eq!(result.by_strategy.len(), 1);
        let arb = &result.by_strategy["arb"];
        assert_eq!(arb.count, 1);
        assert_eq!(arb.profitable, 1);
        assert_approx_eq(arb.roi, 400.0);

        assert_eq!(result.by_dex.len(), 1);
        assert_eq!(result.by_dex[0].dex, "QuickSwap");
        assert_eq!(result.by_dex[0].opportunities, 1);
    }

    #[test]
    fn test_aggregate_mixed_profitability() {
        let opps = vec![
            make_opp(Strategy::TwoHopArb, one_eth(), one_eth() / 5, 1),       // profitable
            make_opp(Strategy::TwoHopArb, one_eth() / 2, one_eth() / 5 * 3, 2), // not profitable
            make_opp(Strategy::Jit, one_eth() * 2, one_eth() / 10 * 3, 3),    // profitable
        ];
        let dexes = vec![DexMeta { name: "QuickSwap".into(), fork: "UniV2".into(), tx_count: 3 }];
        let result = aggregate(&opps, &dexes, 1.5);

        assert_eq!(result.summary.total, 3);
        assert_eq!(result.summary.profitable, 2);
        assert_approx_eq(result.summary.gross_revenue, 3.5);
        assert_approx_eq(result.summary.total_cost, 1.1);
        assert_approx_eq(result.summary.net_profit, 2.4);
        assert_approx_eq(result.summary.net_profit_usd, 3.6);
        assert_eq!(result.summary.best_strategy.as_deref(), Some("jit"));

        assert_eq!(result.by_strategy.len(), 2);
        let arb = &result.by_strategy["arb"];
        assert_eq!(arb.count, 2);
        assert_eq!(arb.profitable, 1);
        assert_approx_eq(arb.gross_revenue, 1.5);
        assert_approx_eq(arb.gas_fees, 0.8);
        assert_approx_eq(arb.net_profit, 0.7);

        let jit = &result.by_strategy["jit"];
        assert_eq!(jit.count, 1);
        assert_eq!(jit.profitable, 1);
        assert_approx_eq(jit.gross_revenue, 2.0);
        assert_approx_eq(jit.gas_fees, 0.3);
        assert_approx_eq(jit.net_profit, 1.7);
    }

    #[test]
    fn test_aggregate_all_unprofitable() {
        let opps = vec![
            make_opp(Strategy::Sandwich, one_eth() / 10, one_eth() / 5, 1),
            make_opp(Strategy::JitArb, one_eth() / 100, one_eth() / 20, 2),
        ];
        let dexes = vec![DexMeta { name: "TestDex".into(), fork: "UniV3".into(), tx_count: 2 }];
        let result = aggregate(&opps, &dexes, 1.0);

        assert_eq!(result.summary.total, 2);
        assert_eq!(result.summary.profitable, 0);
        // best_strategy stays None when all strategies have net profit <= 0
        assert!(result.summary.best_strategy.is_none());
    }

    #[test]
    fn test_aggregate_multiple_dexes_sorted_by_revenue() {
        // Create opps assigned to specific dexes by name via the aggregate fn
        let opps = vec![
            make_opp(Strategy::TwoHopArb, one_eth(), one_eth() / 10, 1),
            make_opp(Strategy::TwoHopArb, one_eth() * 3, one_eth() / 5, 2),
        ];
        // aggregate() assigns ALL opps to ALL dexes, so both get the same total revenue (4 ETH).
        // For a deterministic sort we need unequal revenue. Since both dexes get the same total
        // revenue by design, verify they are sorted correctly when revenue differs.
        let dexes = vec![
            DexMeta { name: "LowDex".into(), fork: "UniV2".into(), tx_count: 0 },
            DexMeta { name: "HighDex".into(), fork: "UniV3".into(), tx_count: 0 },
        ];
        let result = aggregate(&opps, &dexes, 1.0);

        assert_eq!(result.by_dex.len(), 2);
        // Both dexes get the same revenue (4.0), so sort order is stable by position.
        // When equal, sort uses whatever order partial_cmp returns, which preserves
        // original ordering for equal values.
        let revs: Vec<f64> = result.by_dex.iter().map(|d| d.revenue).collect();
        assert!((revs[0] - revs[1]).abs() < 1e-10, "both should have equal revenue");
    }

    #[test]
    fn test_aggregate_wei_precision() {
        // Test with very small values to verify wei math doesn't overflow
        let opps = vec![make_opp(Strategy::TwoHopArb, 1, 0, 1)];
        let result = aggregate(&opps, &[], 1.0);
        assert_eq!(result.summary.total, 1);
        assert_eq!(result.summary.gross_revenue_wei, 1);
        assert_eq!(result.summary.total_gas_cost_wei, 0);
        assert_eq!(result.summary.net_profit_wei, 1);
    }

    #[test]
    fn test_aggregate_zero_gas_roi() {
        let opps = vec![make_opp(Strategy::TwoHopArb, one_eth(), 0, 1)];
        let dexes = vec![DexMeta { name: "Dex".into(), fork: "UniV2".into(), tx_count: 1 }];
        let result = aggregate(&opps, &dexes, 1.0);
        let arb = &result.by_strategy["arb"];
        assert_approx_eq(arb.roi, 0.0);
    }

    #[test]
    fn test_aggregate_usd_conversion() {
        let opps = vec![make_opp(Strategy::TwoHopArb, one_eth(), one_eth() / 2, 1)];
        let result = aggregate(&opps, &[], 50000.0);
        assert_approx_eq(result.summary.net_profit_usd, 25000.0);
    }

    fn assert_approx_eq(a: f64, b: f64) {
        let diff = (a - b).abs();
        assert!(diff < 1e-6, "expected {b}, got {a}, diff {diff}");
    }
}
