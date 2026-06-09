use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::state::AppState;

pub async fn export_json(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let path = PathBuf::from(&state.results_dir).join(format!("{}.json", run_id));
    let content = tokio::fs::read_to_string(&path).await.map_err(|_| {
        (StatusCode::NOT_FOUND, "run not found".to_string())
    })?;

    let headers = [(
        axum::http::header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}.json\"", run_id),
    )];

    Ok((headers, content))
}

pub async fn export_csv(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let path = PathBuf::from(&state.results_dir).join(format!("{}.json", run_id));
    let content = tokio::fs::read_to_string(&path).await.map_err(|_| {
        (StatusCode::NOT_FOUND, "run not found".to_string())
    })?;

    let val: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("parse error: {}", e))
    })?;

    let mut csv_output = String::from("tx_hash,block_number,timestamp,strategy,gross_revenue,gas_cost,flash_loan_fee,builder_tip,net_profit,result,token_pair,dex_path\n");

    if let Some(opps) = val.get("opportunities").and_then(|v| v.as_array()) {
        for opp in opps {
            let tx_hash = opp.get("tx_hash").and_then(|v| v.as_str()).unwrap_or("");
            let block = opp.get("block_number").and_then(|v| v.as_u64()).unwrap_or(0);
            let ts = opp.get("timestamp").and_then(|v| v.as_u64()).unwrap_or(0);
            let strategy = opp.get("strategy").and_then(|v| v.as_str()).unwrap_or("");
            let gross = opp.get("gross_revenue").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let gas = opp.get("gas_cost").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let fl_fee = opp.get("flash_loan_fee").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let tip = opp.get("builder_tip").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let net = opp.get("net_profit").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let result = opp.get("result").and_then(|v| v.as_str()).unwrap_or("");
            let token_pair = opp.get("token_pair").and_then(|v| v.as_str()).unwrap_or("");
            let dex_path = opp.get("dex_path").and_then(|v| v.as_array()).map(|a| {
                a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(";")
            }).unwrap_or_default();

            csv_output.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{}\n",
                tx_hash, block, ts, strategy, gross, gas, fl_fee, tip, net, result, token_pair, dex_path
            ));
        }
    }

    let headers = [(
        axum::http::header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}.csv\"", run_id),
    )];

    Ok((headers, csv_output))
}
