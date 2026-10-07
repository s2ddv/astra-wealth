use crate::{
    auth::{AuthState, AuthenticatedUser},
    wallet::WalletChain,
};
use application::tracking::{TrackedAsset, TrackedWallet, TrackingError, WalletTrackingService};
use axum::{
    Json, Router,
    extract::{FromRef, Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;
use ts_rs::TS;

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WalletAssetDto {
    pub symbol: String,
    pub name: String,
    pub contract_address: Option<String>,
    pub amount: String,
    pub price_usd: Option<String>,
    pub value_usd: Option<String>,
    pub is_native: bool,
}
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WalletAssetsResponse {
    pub wallet_id: String,
    pub address: String,
    pub chain: WalletChain,
    pub total_usd: Option<String>,
    pub complete: bool,
    pub valuation_complete: bool,
    pub assets: Vec<WalletAssetDto>,
    pub updated_at: String,
}
impl From<TrackedAsset> for WalletAssetDto {
    fn from(a: TrackedAsset) -> Self {
        Self {
            symbol: a.symbol,
            name: a.name,
            contract_address: a.contract_address,
            amount: a.amount,
            price_usd: a.price_usd,
            value_usd: a.value_usd,
            is_native: a.is_native,
        }
    }
}
impl TryFrom<TrackedWallet> for WalletAssetsResponse {
    type Error = TrackingHttpError;
    fn try_from(w: TrackedWallet) -> Result<Self, Self::Error> {
        let chain = w
            .chain
            .parse::<domain::wallet::Chain>()
            .map_err(|_| TrackingHttpError(TrackingError::StorageUnavailable))?;
        Ok(Self {
            wallet_id: w.wallet_id,
            address: w.address,
            chain: chain.into(),
            total_usd: w.total_usd,
            complete: w.complete,
            valuation_complete: w.valuation_complete,
            assets: w.assets.into_iter().map(Into::into).collect(),
            updated_at: w.updated_at,
        })
    }
}
#[derive(Clone)]
struct TrackingState {
    auth: AuthState,
    service: Arc<WalletTrackingService>,
}
impl FromRef<TrackingState> for AuthState {
    fn from_ref(state: &TrackingState) -> Self {
        state.auth.clone()
    }
}
pub fn router(auth: AuthState, service: WalletTrackingService) -> Router {
    Router::new()
        .route("/v1/me/wallets/{id}/assets", get(assets))
        .route("/v1/me/wallets/{id}/sync", post(sync))
        .with_state(TrackingState {
            auth,
            service: Arc::new(service),
        })
}
async fn assets(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<TrackingState>,
    Path(id): Path<String>,
) -> Result<Json<WalletAssetsResponse>, TrackingHttpError> {
    Ok(Json(
        state
            .service
            .assets(&user.id, &id, false)
            .await?
            .try_into()?,
    ))
}
async fn sync(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<TrackingState>,
    Path(id): Path<String>,
) -> Result<Json<WalletAssetsResponse>, TrackingHttpError> {
    Ok(Json(
        state
            .service
            .assets(&user.id, &id, true)
            .await?
            .try_into()?,
    ))
}
#[derive(Debug)]
pub struct TrackingHttpError(pub TrackingError);
impl From<TrackingError> for TrackingHttpError {
    fn from(e: TrackingError) -> Self {
        Self(e)
    }
}
impl IntoResponse for TrackingHttpError {
    fn into_response(self) -> Response {
        let (status, code) = match self.0 {
            TrackingError::NotFound => (StatusCode::NOT_FOUND, "WALLET_NOT_FOUND"),
            TrackingError::ChainNotSupported => {
                (StatusCode::UNPROCESSABLE_ENTITY, "CHAIN_NOT_SUPPORTED")
            }
            TrackingError::ProviderUnavailable => (StatusCode::BAD_GATEWAY, "PROVIDER_UNAVAILABLE"),
            TrackingError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED"),
            TrackingError::StorageUnavailable => {
                (StatusCode::SERVICE_UNAVAILABLE, "TRACKING_UNAVAILABLE")
            }
        };
        let mut response = (
            status,
            Json(json!({"error": self.0.to_string(), "code":code})),
        )
            .into_response();
        if status == StatusCode::TOO_MANY_REQUESTS {
            response.headers_mut().insert(
                header::RETRY_AFTER,
                axum::http::HeaderValue::from_static("15"),
            );
        }
        response
    }
}
