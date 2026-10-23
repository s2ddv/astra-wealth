use application::public_wallet::PublicWalletService;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use domain::asset_icon::IconError;
use serde_json::json;

pub fn router(service: PublicWalletService) -> Router {
    Router::new()
        .route("/v1/wallet/{address}", get(summary))
        .with_state(service)
}
async fn summary(
    State(service): State<PublicWalletService>,
    Path(address): Path<String>,
) -> Response {
    match service.summary(&address).await {
        Ok(summary) => Json(summary).into_response(),
        Err(IconError::InvalidReference) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"Invalid Ethereum address"})),
        )
            .into_response(),
        Err(_) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({"error":"Failed to fetch wallet data"})),
        )
            .into_response(),
    }
}
