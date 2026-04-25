use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use resonate::prelude::*;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Deserialize)]
struct StartReq {
    workflow_id: String,
}

#[derive(Clone)]
struct AppState {
    resonate: Arc<Resonate>,
}

#[tokio::main]
async fn main() {
    let resonate = Resonate::new(ResonateConfig {
        url: Some("http://localhost:8001".into()),
        group: Some("gateway".into()),
        ..Default::default()
    });

    let state = AppState {
        resonate: Arc::new(resonate),
    };

    let app = Router::new()
        .route("/start-workflow", post(start_workflow))
        .route("/resolve/:promise_id", get(resolve_promise))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:5001")
        .await
        .expect("failed to bind 127.0.0.1:5001");
    println!("Gateway listening on http://127.0.0.1:5001");
    axum::serve(listener, app)
        .await
        .expect("axum server crashed");
}

/// Start (or reconnect to) a workflow.
///
/// Resonate deduplicates by promise ID: invoking with a `workflow_id` that
/// already has a PENDING execution reconnects to it; one that has RESOLVED
/// returns the cached result. Either way, the gateway just awaits the result.
async fn start_workflow(
    State(state): State<AppState>,
    Json(req): Json<StartReq>,
) -> impl IntoResponse {
    let result: String = state
        .resonate
        .rpc(&req.workflow_id, "foo", req.workflow_id.clone())
        .target("poll://any@workers")
        .await
        .expect("rpc to worker failed");

    Json(json!({ "message": result }))
}

/// Resolve the latent promise — this is what unblocks the workflow.
///
/// The promise ID is exactly the one printed by the worker's `send_email`
/// step. Hitting this endpoint resolves it, and any worker currently
/// suspended on `blocking_promise.await` resumes and finishes the workflow.
async fn resolve_promise(
    State(state): State<AppState>,
    Path(promise_id): Path<String>,
) -> impl IntoResponse {
    state
        .resonate
        .promises
        .resolve(&promise_id, json!(true))
        .await
        .expect("failed to resolve promise");

    Json(json!({ "message": "promise resolved" }))
}
