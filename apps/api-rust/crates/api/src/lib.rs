//! HTTP boundary for health and authenticated wallet CRUD.
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
        service: "astra-wealth-api",
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

pub mod wallet;

pub fn router(
    health: HealthState,
    auth: auth::AuthState,
    wallets: application::wallet::WalletService,
) -> Router {
    // Watchlist will be composed here using the same AuthState and its own service.
    health_router(health).merge(wallet::router(wallet::WalletState {
        auth,
        wallets: Arc::new(wallets),
    }))
}

pub mod asset_icon;

pub mod market;

pub mod news;
