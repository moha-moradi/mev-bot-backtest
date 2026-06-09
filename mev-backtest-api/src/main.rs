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

    let cors = CorsLayer::new()
        .allow_origin(HeaderValue::from_static("http://localhost:8080"))
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
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
        .with_state(app_state);

    let addr = "0.0.0.0:3001";
    info!("MEVSCOPE API listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}
