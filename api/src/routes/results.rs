use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use axum::http::StatusCode;

use crate::state::AppState;

pub async fn get_results(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let runs = state.runs.read().await;
    let run_state = runs.get(&run_id).ok_or_else(|| {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "run not found"})))
    })?;

    let run = run_state.read().await;
    match &run.result {
        Some(result) => {
            Ok(Json(serde_json::json!({
                "run_id": result.run_id,
                "chain": result.chain,
                "start_block": result.start_block,
                "end_block": result.end_block,
                "strategies": result.strategies,
                "opportunities": result.opportunities,
                "summary": result.summary,
                "by_strategy": result.by_strategy,
                "by_dex": result.by_dex,
                "duration_ms": result.duration_ms,
                "created_at": result.created_at,
            })))
        }
        None => {
            let status = match &run.status {
                crate::state::RunStatus::Done => "done",
                crate::state::RunStatus::Running => "running",
                crate::state::RunStatus::Pending => "pending",
                crate::state::RunStatus::Error(msg) => { return Ok(Json(serde_json::json!({ "run_id": run.run_id, "status": "error", "error": msg, "logs": run.logs }))); }
                crate::state::RunStatus::Cancelled => "cancelled",
            };
            Ok(Json(serde_json::json!({
                "run_id": run.run_id,
                "status": status,
                "progress": run.progress,
                "blocks_processed": run.blocks_processed,
                "blocks_total": run.blocks_total,
                "stages": run.stages,
                "logs": run.logs,
                "message": "results not available yet"
            })))
        }
    }
}
