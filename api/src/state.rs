use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use mev_scout_core::config::Config;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SseEvent {
    pub event_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogEntry {
    pub ts: String,
    pub tag: String,
    pub text: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StageState {
    pub id: String,
    pub label: String,
    pub status: StageStatus,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum StageStatus {
    Pending,
    Running,
    Completed,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RunStatus {
    Pending,
    Running,
    Done,
    Error(String),
    Cancelled,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RunResult {
    pub run_id: String,
    pub chain: String,
    pub start_block: u64,
    pub end_block: u64,
    pub range_mode: String,
    pub strategies: Vec<String>,
    pub opportunities: Vec<UiOpportunity>,
    pub summary: Option<mev_scout_core::aggregate::SummaryMetrics>,
    pub by_strategy: Option<std::collections::HashMap<String, mev_scout_core::aggregate::StrategyMetrics>>,
    pub by_dex: Option<Vec<mev_scout_core::aggregate::DexMetrics>>,
    pub duration_ms: u64,
    pub created_at: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UiOpportunity {
    pub id: String,
    pub tx_hash: String,
    pub block_number: u64,
    pub timestamp: u64,
    pub strategy: String,
    pub gross_revenue: f64,
    pub gas_cost: f64,
    pub flash_loan_fee: f64,
    pub builder_tip: f64,
    pub net_profit: f64,
    pub net_profit_usd: f64,
    pub result: String,
    pub explorer_url: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_pair: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dex_path: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_a: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_b: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_amount: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub flash_loan_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flash_loan_size: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub victim_tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub front_run_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub victim_slippage: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gross_capture: Option<f64>,

    pub simulation_trace: SimulationTrace,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimulationTrace {
    pub title: String,
    pub steps: Vec<TraceStep>,
    pub result: TraceResult,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TraceStep {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TraceResult {
    pub gross: f64,
    pub cost: f64,
    pub net: f64,
}

#[derive(Debug, Clone)]
pub struct RunState {
    pub run_id: String,
    pub config: Config,
    pub status: RunStatus,
    pub stages: Vec<StageState>,
    pub logs: Vec<LogEntry>,
    pub progress: f64,
    pub elapsed_ms: u64,
    pub started_at: u64,
    pub sse_tx: broadcast::Sender<SseEvent>,
    pub result: Option<RunResult>,
    pub blocks_processed: u64,
    pub blocks_total: u64,
}

pub struct AppState {
    pub runs: RwLock<HashMap<String, Arc<RwLock<RunState>>>>,
    pub results_dir: String,
}
