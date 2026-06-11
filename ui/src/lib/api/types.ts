export interface ApiDexConfig {
  id: string;
  name: string;
  fork: string;
  router: string;
}

export interface ApiChain {
  id: string;
  name: string;
  native_token: string;
  color: string;
  block_time: number;
  rpc_default: string;
  explorer_base: string;
  coingecko_id: string;
  activity_multiplier: number;
  avg_tx_per_block: number;
  gas_price_gwei: number;
  native_usd: number;
  dexes: ApiDexConfig[];
  flash_loan_providers: string[];
}

export type WindowMode = "single" | "days" | "blocks" | "range";

export interface WindowConfig {
  mode: WindowMode;
  single_block?: number;
  last_days?: number;
  from_block?: number;
  to_block?: number;
}

export interface SimulateRequest {
  chain: string;
  rpc_url?: string;
  window?: WindowConfig;
  strategies: string[];
  flash_loan_provider?: string;
  gas_model?: string;
  priority_fee_gwei?: number;
  gas_limit?: number;
}

export interface SimulateResponse {
  run_id: string;
  status: string;
  created_at: number;
}

export interface ApiSimulationTraceStep {
  label: string;
  value?: string | null;
  sub?: string | null;
}

export interface ApiSimulationTrace {
  title: string;
  steps: ApiSimulationTraceStep[];
  result: { gross: number; cost: number; net: number };
}

export interface ApiUiOpportunity {
  id: string;
  tx_hash: string;
  block_number: number;
  timestamp: number;
  strategy: string;
  gross_revenue: number;
  gas_cost: number;
  flash_loan_fee: number;
  builder_tip: number;
  net_profit: number;
  result: string;
  explorer_url: string;
  token_pair?: string | null;
  dex_path?: string[] | null;
  pool_a?: string | null;
  pool_b?: string | null;
  input_amount?: string | null;
  flash_loan_provider?: string | null;
  flash_loan_size?: number | null;
  victim_tx_hash?: string | null;
  front_run_size?: number | null;
  victim_slippage?: number | null;
  gross_capture?: number | null;
  simulation_trace: ApiSimulationTrace;
}

export interface ApiSummaryMetrics {
  total: number;
  profitable: number;
  gross_revenue: number;
  net_profit: number;
  net_profit_usd: number;
  total_cost: number;
  best_strategy?: string | null;
  best_single_opp: number;
}

export interface ApiStrategyMetrics {
  strategy: string;
  count: number;
  profitable: number;
  gross_revenue: number;
  gas_fees: number;
  net_profit: number;
  net_profit_usd: number;
  roi: number;
  avg_per_opp: number;
  best_opp: number;
}

export interface ApiDexMetrics {
  dex: string;
  fork: string;
  tx_count: number;
  opportunities: number;
  profitable: number;
  revenue: number;
  avg_profit: number;
}

export interface ApiStageStatus {
  id: string;
  label: string;
  status: string;
}

export interface ApiLogEntry {
  ts: string;
  tag: string;
  text: string;
}

export interface ApiResultsCompleted {
  run_id: string;
  chain: string;
  start_block: number;
  end_block: number;
  strategies: string[];
  opportunities: ApiUiOpportunity[];
  summary: ApiSummaryMetrics;
  by_strategy: Record<string, ApiStrategyMetrics>;
  by_dex: ApiDexMetrics[];
  duration_ms: number;
  created_at: number;
}

export interface ApiResultsRunning {
  run_id: string;
  status: "running" | "pending";
  progress: number;
  blocks_processed: number;
  blocks_total: number;
  stages: ApiStageStatus[];
  logs: ApiLogEntry[];
  message: string;
}

export interface ApiResultsError {
  run_id: string;
  status: "error";
  error: string;
  logs: ApiLogEntry[];
}

export type ApiResultsResponse = ApiResultsCompleted | ApiResultsRunning | ApiResultsError;

export interface ApiHistoryEntry {
  id: string;
  started_at: number;
  duration_ms: number;
  chain_id: string;
  window_summary: string;
  enabled_strategies: string[];
  opportunities: number;
  net_profit: number;
}

export interface ApiDeleteResponse {
  deleted: string;
}

export interface ApiError {
  error: string;
}

export interface SseStageStart {
  stage: number;
  id: string;
  label: string;
  sub: string;
}

export interface SseStageEnd {
  stage: number;
  id: string;
  result: string;
}

export interface SseProgress {
  stage: number;
  block: number;
  blocks_processed: number;
  total_blocks: number;
}

export interface SseLog {
  ts: string;
  tag: string;
  text: string;
}

export interface SseComplete {
  run_id: string;
  duration_ms: number;
}

export interface SseError {
  stage: number;
  id: string;
  error: string;
}

export type SseEvent = { type: "stage_start"; data: SseStageStart }
  | { type: "stage_end"; data: SseStageEnd }
  | { type: "progress"; data: SseProgress }
  | { type: "log"; data: SseLog }
  | { type: "complete"; data: SseComplete }
  | { type: "error"; data: SseError };
