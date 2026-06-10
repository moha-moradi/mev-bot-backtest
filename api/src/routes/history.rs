use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use crate::state::AppState;

#[derive(Debug, serde::Serialize)]
pub struct HistoryEntry {
    pub id: String,
    pub started_at: u64,
    pub duration_ms: u64,
    pub chain_id: String,
    pub window_summary: String,
    pub enabled_strategies: Vec<String>,
    pub opportunities: usize,
    pub net_profit: f64,
}

pub async fn list_history(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<HistoryEntry>>, (StatusCode, Json<serde_json::Value>)> {
    let dir = PathBuf::from(&state.results_dir);
    let mut entries = Vec::new();

    if !dir.exists() {
        return Ok(Json(entries));
    }

    let mut read_dir = tokio::fs::read_dir(&dir).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("failed to read results dir: {}", e)})))
    })?;

    while let Some(entry) = read_dir.next_entry().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("failed to read entry: {}", e)})))
    })? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let content = tokio::fs::read_to_string(&path).await.ok();
        if let Some(json_str) = content {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
                let id = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                entries.push(HistoryEntry {
                    id,
                    started_at: val.get("created_at").and_then(|v| v.as_u64()).unwrap_or(0),
                    duration_ms: val.get("duration_ms").and_then(|v| v.as_u64()).unwrap_or(0),
                    chain_id: val.get("chain").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    window_summary: val.get("range_mode").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    enabled_strategies: val.get("strategies").and_then(|v| v.as_array()).map(|a| {
                        a.iter().filter_map(|v| v.as_str().map(String::from)).collect()
                    }).unwrap_or_default(),
                    opportunities: val.get("opportunities").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
                    net_profit: 0.0,
                });
            }
        }
    }

    entries.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(Json(entries))
}

pub async fn get_history_run(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let path = PathBuf::from(&state.results_dir).join(format!("{}.json", run_id));
    let content = tokio::fs::read_to_string(&path).await.map_err(|_| {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "run not found"})))
    })?;
    let val: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("parse error: {}", e)})))
    })?;
    Ok(Json(val))
}

pub async fn delete_history_run(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let path = PathBuf::from(&state.results_dir).join(format!("{}.json", run_id));
    if !path.exists() {
        return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "run not found"}))));
    }
    tokio::fs::remove_file(&path).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("delete failed: {}", e)})))
    })?;
    Ok(Json(serde_json::json!({"deleted": run_id})))
}
