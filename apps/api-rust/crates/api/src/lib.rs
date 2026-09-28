//! HTTP boundary. Wallet routes will compose with this router in the next stage.
use axum::{Json, Router, extract::State, routing::get};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use std::sync::Arc;

#[derive(Clone)]
pub struct HealthState {
    pub dependencies: Arc<infrastructure::HealthDependencies>,
}

#[derive(Serialize)]
pub struct HealthResponse {
    status: &'static str,
    service: &'static str,
    timestamp: String,
    checks: Checks,
}
#[derive(Serialize)]
struct Checks {
    database: &'static str,
    redis: &'static str,
}

async fn health(State(state): State<HealthState>) -> Json<HealthResponse> {
    let (database, redis) = state.dependencies.check().await;
    Json(HealthResponse {
        status: if database && redis { "ok" } else { "degraded" },
        service: "zora-wealth-api",
        timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        checks: Checks {
            database: if database { "ok" } else { "error" },
            redis: if redis { "ok" } else { "error" },
        },
    })
}

pub fn health_router(state: HealthState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/health", get(health))
        .route("/v1/health/", get(health))
        .with_state(state)
}

pub mod auth;
pub mod user;

pub fn router(health: HealthState, auth: auth::AuthState) -> Router {
    // Next: merge wallet::router().with_state(auth.clone()) here. The state is
    // wired now so wallet handlers can use AuthenticatedUser without changes.
    let protected: Router<auth::AuthState> = Router::new();
    health_router(health).merge(protected.with_state(auth))
}

pub mod asset_icon;

pub mod market;

pub mod news;
