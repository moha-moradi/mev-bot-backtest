use std::sync::Arc;
use std::convert::Infallible;

use axum::extract::{Path, State};
use axum::response::sse::{Event, Sse};
use axum::response::IntoResponse;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::state::AppState;

pub async fn stream_status(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> impl IntoResponse {
    let runs = state.runs.read().await;
    let run_state = match runs.get(&run_id) {
        Some(rs) => rs.clone(),
        None => {
            return Err((
                axum::http::StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({"error": "run not found"})),
            ));
        }
    };
    drop(runs);

    let rx = run_state.read().await.sse_tx.subscribe();

    let stream = BroadcastStream::new(rx).filter_map(|result| {
        match result {
            Ok(sse) => {
                let data = serde_json::to_string(&sse.data).unwrap_or_default();
                Some(Ok::<Event, Infallible>(
                    Event::default()
                        .event(sse.event_type)
                        .data(data)
                ))
            }
            Err(_) => None,
        }
    });

    Ok(Sse::new(stream))
}
