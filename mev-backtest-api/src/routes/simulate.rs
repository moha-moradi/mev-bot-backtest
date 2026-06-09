use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::State;
use axum::Json;
use mev_backtest_core::config::Config;
use mev_backtest_core::types::{RangeMode, Strategy};
use tokio::sync::RwLock;
use tracing::info;

use crate::pipeline::{PipelineParams, run_pipeline};
use crate::state::{
    AppState, RunState, RunStatus, SseEvent, StageState, StageStatus,
};

#[derive(Debug, serde::Deserialize)]
pub struct WindowConfig {
    pub mode: String,
    pub last_days: Option<u64>,
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
    pub single_block: Option<u64>,
}

#[derive(Debug, serde::Deserialize)]
pub struct SimulateRequest {
    pub chain: String,
    pub rpc_url: Option<String>,
    pub window: Option<WindowConfig>,
    pub strategies: Vec<String>,
    pub flash_loan_provider: Option<String>,
    pub gas_model: Option<String>,
    pub priority_fee_gwei: Option<f64>,
    pub gas_limit: Option<u64>,
}

#[derive(Debug, serde::Serialize)]
pub struct SimulateResponse {
    pub run_id: String,
    pub status: String,
    pub created_at: u64,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn make_run_id() -> String {
    format!("run_{}", now_secs())
}

fn map_strategy(ui_name: &str) -> Option<Strategy> {
    match ui_name {
        "arb" => Some(Strategy::TwoHopArb),
        "jit" => Some(Strategy::Jit),
        "jitarb" => Some(Strategy::JitArb),
        "sandwich" => Some(Strategy::Sandwich),
        _ => None,
    }
}

fn build_range_mode(window: &WindowConfig) -> Result<RangeMode, String> {
    match window.mode.as_str() {
        "days" => {
            let d = window.last_days.unwrap_or(7);
            if !(1..=365).contains(&d) {
                return Err("days must be between 1 and 365".into());
            }
            Ok(RangeMode::Days(d))
        }
        "blocks" => {
            let b = window.last_days.unwrap_or(100);
            if b < 1 {
                return Err("blocks must be >= 1".into());
            }
            Ok(RangeMode::Blocks(b))
        }
        "single" => {
            let b = window.single_block.unwrap_or(0);
            if b == 0 {
                return Err("single_block must be > 0".into());
            }
            Ok(RangeMode::Single(b))
        }
        "range" => {
            let from = window.from_block.unwrap_or(0);
            let to = window.to_block.unwrap_or(0);
            if to <= from {
                return Err("to_block must be greater than from_block".into());
            }
            Ok(RangeMode::Range(from, to))
        }
        _ => Err(format!("unknown window mode '{}'", window.mode)),
    }
}

fn chain_config_for(chain: &str) -> Option<mev_backtest_core::config::ChainConfig> {
    let config = Config::default();
    config.chains.get(chain).cloned()
}

pub async fn simulate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SimulateRequest>,
) -> Result<Json<SimulateResponse>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let valid_chains = [
        "ethereum", "polygon", "bsc", "arbitrum", "avalanche", "base", "optimism",
    ];
    if !valid_chains.contains(&req.chain.as_str()) {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("unknown chain '{}'. Supported: {}", req.chain, valid_chains.join(", "))
            })),
        ));
    }

    let chain_config = chain_config_for(&req.chain).ok_or_else(|| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("chain '{}' not configured", req.chain)})),
        )
    })?;

    let rpc_url = req.rpc_url.unwrap_or_else(|| format!("https://{}.rpc.example.com", req.chain));

    let range_mode = match &req.window {
        Some(w) => build_range_mode(w).map_err(|e| {
            (axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e})))
        })?,
        None => RangeMode::Blocks(100),
    };

    let strategies: Vec<Strategy> = req
        .strategies
        .iter()
        .filter_map(|s| {
            let mapped = map_strategy(s);
            if mapped.is_none() {
                info!("Strategy '{}' not implemented, skipping", s);
            }
            mapped
        })
        .collect();

    if strategies.is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "at least one valid strategy required (arb, jit, jitarb, sandwich)"})),
        ));
    }

    let flash_loan_provider = req.flash_loan_provider.unwrap_or_else(|| "auto".to_string());
    let gas_model = req.gas_model.unwrap_or_else(|| "historical_exact".to_string());
    let priority_fee_gwei = req.priority_fee_gwei.unwrap_or(0.0);
    let gas_limit = req.gas_limit.unwrap_or(200_000);

    let (sse_tx, _) = tokio::sync::broadcast::channel::<SseEvent>(256);
    let run_id = make_run_id();

    let stages = vec![
        StageState { id: "rpc_fetch".into(), label: "RPC FETCH".into(), status: StageStatus::Pending },
        StageState { id: "tx_filter".into(), label: "TX FILTER".into(), status: StageStatus::Pending },
        StageState { id: "revm_replay".into(), label: "REVM REPLAY".into(), status: StageStatus::Pending },
        StageState { id: "opportunity_scan".into(), label: "OPPORTUNITY SCAN".into(), status: StageStatus::Pending },
        StageState { id: "profitability".into(), label: "PROFITABILITY CHECK".into(), status: StageStatus::Pending },
        StageState { id: "aggregation".into(), label: "AGGREGATION".into(), status: StageStatus::Pending },
    ];

    let run_state = Arc::new(RwLock::new(RunState {
        run_id: run_id.clone(),
        config: Config::default(),
        status: RunStatus::Pending,
        stages,
        logs: Vec::new(),
        progress: 0.0,
        elapsed_ms: 0,
        started_at: now_secs(),
        sse_tx: sse_tx.clone(),
        result: None,
        blocks_processed: 0,
        blocks_total: 0,
    }));

    {
        let mut runs = state.runs.write().await;
        runs.insert(run_id.clone(), run_state.clone());
    }

    let params = PipelineParams {
        chain: req.chain.clone(),
        rpc_url,
        range_mode,
        strategies,
        flash_loan_provider,
        gas_model,
        priority_fee_gwei,
        gas_limit,
        cache_dir: format!("./cache/{}", req.chain),
    };

    let sse_tx_clone = sse_tx.clone();
    let run_state_clone = run_state.clone();
    let run_id_clone = run_id.clone();
    let results_dir = state.results_dir.clone();

    tokio::spawn(async move {
        run_pipeline(
            params,
            chain_config,
            run_id_clone,
            sse_tx_clone,
            run_state_clone,
            results_dir,
        ).await;
    });

    Ok(Json(SimulateResponse {
        run_id,
        status: "pending".to_string(),
        created_at: now_secs(),
    }))
}
