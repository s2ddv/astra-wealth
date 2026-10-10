use crate::auth::{AuthState, AuthenticatedUser};
use application::{
    fx::{FxError, FxService, brasilia_today},
    portfolio::PortfolioService,
};
use axum::{
    Json, Router,
    extract::{FromRef, Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::{NaiveDate, Utc};
use serde_json::json;

#[derive(Clone)]
pub struct PortfolioState {
    pub auth: AuthState,
    pub portfolio: PortfolioService,
    pub fx: FxService,
}
impl FromRef<PortfolioState> for AuthState {
    fn from_ref(state: &PortfolioState) -> Self {
        state.auth.clone()
    }
}
pub fn router(state: PortfolioState) -> Router {
    Router::new()
        .route("/v1/me/portfolio", get(current))
        .route("/v1/me/portfolio/snapshots", get(history).post(capture))
        .route("/v1/me/fx/usd-brl", get(current_fx))
        .route("/v1/me/fx/usd-brl/{date}", get(historical_fx))
        .layer(axum::middleware::map_response(
            |mut response: Response| async move {
                response
                    .headers_mut()
                    .insert(header::CACHE_CONTROL, "private, no-store".parse().unwrap());
                response
            },
        ))
        .with_state(state)
}
fn unavailable() -> Response {
    (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"code":"PORTFOLIO_UNAVAILABLE","error":"Consolidação indisponível. Tente novamente."}))).into_response()
}
async fn current(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<PortfolioState>,
) -> Response {
    match tokio::time::timeout(
        std::time::Duration::from_secs(50),
        state.portfolio.current(&user.id, Utc::now()),
    )
    .await
    {
        Ok(Ok(value)) => Json(value).into_response(),
        _ => unavailable(),
    }
}
async fn capture(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<PortfolioState>,
) -> Response {
    match tokio::time::timeout(
        std::time::Duration::from_secs(50),
        state.portfolio.capture(&user.id, Utc::now()),
    )
    .await
    {
        Ok(Ok(value)) => (StatusCode::CREATED, Json(value)).into_response(),
        _ => unavailable(),
    }
}
async fn history(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<PortfolioState>,
) -> Response {
    match state.portfolio.history(&user.id).await {
        Ok(value) => Json(value).into_response(),
        Err(_) => unavailable(),
    }
}
async fn current_fx(_: AuthenticatedUser, State(state): State<PortfolioState>) -> Response {
    fx_response(&state.fx, brasilia_today(Utc::now())).await
}
async fn historical_fx(
    _: AuthenticatedUser,
    State(state): State<PortfolioState>,
    Path(date): Path<String>,
) -> Response {
    match NaiveDate::parse_from_str(&date, "%Y-%m-%d") {
        Ok(day) if day.to_string() == date => fx_response(&state.fx, day).await,
        _ => (
            StatusCode::BAD_REQUEST,
            Json(json!({"code":"INVALID_DATE","error":"Use YYYY-MM-DD."})),
        )
            .into_response(),
    }
}
async fn fx_response(service: &FxService, date: NaiveDate) -> Response {
    match service.rate(date, Utc::now()).await {
        Ok(value) => Json(value).into_response(),
        Err(FxError::InvalidDate) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"code":"INVALID_DATE","error":"Data fora do intervalo permitido."})),
        )
            .into_response(),
        Err(_) => unavailable(),
    }
}
