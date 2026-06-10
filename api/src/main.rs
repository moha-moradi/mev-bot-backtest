mod mapping;
mod pipeline;
mod routes;
mod state;

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use axum::http::HeaderValue;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let results_dir = std::env::var("RESULTS_DIR").unwrap_or_else(|_| "./results".to_string());
    if let Err(e) = std::fs::create_dir_all(&results_dir) {
        eprintln!("Warning: failed to create results dir '{}': {}", results_dir, e);
    }

    let app_state = Arc::new(AppState {
        runs: Default::default(),
        results_dir,
    });

    let app = make_app(app_state);

    let addr = "0.0.0.0:3001";
    info!("MEVSCOPE API listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}

fn make_app(app_state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(HeaderValue::from_static("http://localhost:8080"))
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/health", get(health_check))
        .route("/api/chains", get(routes::chains::get_chains))
        .route("/api/simulate", post(routes::simulate::simulate))
        .route("/api/simulate/{run_id}/status", get(routes::status::stream_status))
        .route("/api/simulate/{run_id}/results", get(routes::results::get_results))
        .route("/api/history", get(routes::history::list_history))
        .route("/api/history/{run_id}", get(routes::history::get_history_run).delete(routes::history::delete_history_run))
        .route("/api/export/{run_id}/json", get(routes::export::export_json))
        .route("/api/export/{run_id}/csv", get(routes::export::export_csv))
        .layer(cors)
        .with_state(app_state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn test_app() -> Router {
        let state = Arc::new(AppState {
            runs: Default::default(),
            results_dir: std::env::temp_dir().join("mevscope_test_results").to_string_lossy().to_string(),
        });
        make_app(state)
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = test_app();
        let response = app
            .oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"OK");
    }

    #[tokio::test]
    async fn test_chains_endpoint() {
        let app = test_app();
        let response = app
            .oneshot(Request::builder().uri("/api/chains").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let chains: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(!chains.is_empty(), "Should return at least one chain");
        let ids: Vec<&str> = chains.iter().map(|c| c["id"].as_str().unwrap()).collect();
        assert!(ids.contains(&"polygon"));
        assert!(ids.contains(&"ethereum"));
    }

    #[tokio::test]
    async fn test_results_returns_404_for_unknown_run() {
        let app = test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/simulate/nonexistent-run-id/results")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }
}
