import type { ChainConfig, DexConfig, StrategyId } from "@/lib/chains";
import type { Opportunity, SummaryMetrics, StrategyMetrics, DexMetrics, SimulationTrace } from "@/lib/mockData";
import type { SimulationRun } from "@/store/simulationStore";
import type {
  ApiChain,
  ApiUiOpportunity,
  ApiSummaryMetrics,
  ApiStrategyMetrics,
  ApiDexMetrics,
  ApiHistoryEntry,
  ApiSimulationTrace,
  ApiResultsCompleted,
} from "./types";

export function adaptChain(api: ApiChain): ChainConfig {
  return {
    id: api.id,
    name: api.name,
    nativeToken: api.native_token,
    color: api.color,
    blockTime: api.block_time,
    rpcDefault: api.rpc_default,
    explorerBase: api.explorer_base,
    coingeckoId: api.coingecko_id,
    activityMultiplier: api.activity_multiplier,
    avgTxPerBlock: api.avg_tx_per_block,
    gasPriceGwei: api.gas_price_gwei,
    nativeUSD: api.native_usd,
    dexes: api.dexes.map(adaptDexConfig),
    flashLoanProviders: api.flash_loan_providers,
    lendingProtocols: [],
  };
}

function adaptDexConfig(api: { id: string; name: string; fork: string; router: string }): DexConfig {
  return {
    id: api.id,
    name: api.name,
    fork: api.fork as DexConfig["fork"],
    router: api.router,
  };
}

export function adaptOpportunity(api: ApiUiOpportunity): Opportunity {
  return {
    id: api.id,
    txHash: api.tx_hash,
    blockNumber: api.block_number,
    timestamp: api.timestamp,
    strategy: api.strategy as StrategyId,
    grossRevenue: api.gross_revenue,
    gasCost: api.gas_cost,
    flashLoanFee: api.flash_loan_fee,
    builderTip: api.builder_tip,
    netProfit: api.net_profit,
    result: api.result as Opportunity["result"],
    explorerUrl: api.explorer_url,
    dexPath: api.dex_path ?? undefined,
    tokenPair: api.token_pair ?? undefined,
    flashLoanProvider: api.flash_loan_provider ?? undefined,
    flashLoanSize: api.flash_loan_size ?? undefined,
    victimTxHash: api.victim_tx_hash ?? undefined,
    frontRunSize: api.front_run_size ?? undefined,
    victimSlippage: api.victim_slippage ?? undefined,
    grossCapture: api.gross_capture ?? undefined,
    simulationTrace: adaptSimulationTrace(api.simulation_trace),
  };
}

function adaptSimulationTrace(api: ApiSimulationTrace): SimulationTrace {
  return {
    title: api.title,
    steps: api.steps.map((s) => ({
      label: s.label,
      value: s.value ?? undefined,
      sub: s.sub ?? undefined,
    })),
    result: { gross: api.result.gross, cost: api.result.cost, net: api.result.net },
  };
}

export function adaptSummary(api: ApiSummaryMetrics): SummaryMetrics {
  return {
    total: api.total,
    profitable: api.profitable,
    grossRevenue: api.gross_revenue,
    netProfit: api.net_profit,
    netProfitUSD: api.net_profit_usd,
    totalCost: api.total_cost,
    bestStrategy: (api.best_strategy ?? null) as StrategyId | null,
    bestSingleOpp: api.best_single_opp,
  };
}

export function adaptStrategyMetrics(api: ApiStrategyMetrics): StrategyMetrics {
  return {
    strategy: api.strategy as StrategyId,
    count: api.count,
    profitable: api.profitable,
    grossRevenue: api.gross_revenue,
    gasFees: api.gas_fees,
    netProfit: api.net_profit,
    netProfitUSD: api.net_profit_usd,
    roi: api.roi,
    avgPerOpp: api.avg_per_opp,
    bestOpp: api.best_opp,
  };
}

export function adaptDexMetrics(api: ApiDexMetrics): DexMetrics {
  return {
    dex: api.dex,
    fork: api.fork,
    txCount: api.tx_count,
    opportunities: api.opportunities,
    profitable: api.profitable,
    revenue: api.revenue,
    avgProfit: api.avg_profit,
  };
}

export function adaptHistoryEntry(api: ApiHistoryEntry): SimulationRun {
  return {
    id: api.id,
    startedAt: api.started_at * 1000,
    durationMs: api.duration_ms,
    chainId: api.chain_id,
    windowSummary: api.window_summary,
    enabledStrategies: api.enabled_strategies as StrategyId[],
    opportunities: api.opportunities,
    netProfit: api.net_profit,
  } as SimulationRun;
}

export function adaptCompletedResults(api: ApiResultsCompleted, chainId: string) {
  return {
    opportunities: api.opportunities.map(adaptOpportunity),
    summary: adaptSummary(api.summary),
    byStrategy: Object.fromEntries(
      Object.entries(api.by_strategy).map(([k, v]) => [k, adaptStrategyMetrics(v)])
    ),
    byDex: api.by_dex.map(adaptDexMetrics),
    liquidationAnalytics: null,
    longtailAnalytics: null,
  };
}

export const API_STAGE_LABELS: Record<string, string> = {
  rpc_fetch: "RPC FETCH",
  tx_filter: "TX FILTER",
  revm_replay: "REVM REPLAY",
  opportunity_scan: "OPPORTUNITY SCAN",
  profitability: "PROFITABILITY CHECK",
  aggregation: "AGGREGATION",
};

export const API_STAGE_SUB: Record<string, string> = {
  rpc_fetch: "Connecting to RPC, resolving block range",
  tx_filter: "Fetching blocks (cache-first)",
  revm_replay: "Initializing pools, building replayer",
  opportunity_scan: "Scanning each block for MEV",
  profitability: "Filtering opportunities with profit > 0",
  aggregation: "Computing metrics, saving to disk",
};
