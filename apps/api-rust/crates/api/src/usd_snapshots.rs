use crate::auth::{AuthState, AuthenticatedUser};
use application::usd_snapshots::{SnapshotError, UsdSnapshotService};
use axum::{
    Json, Router,
    extract::{
        DefaultBodyLimit, FromRef, Query, State,
        rejection::{JsonRejection, QueryRejection},
    },
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use serde_json::{Value, json};
#[derive(Clone)]
pub struct UsdSnapshotState {
    pub auth: AuthState,
    pub snapshots: UsdSnapshotService,
}
impl FromRef<UsdSnapshotState> for AuthState {
    fn from_ref(state: &UsdSnapshotState) -> Self {
        state.auth.clone()
    }
}
pub fn router(state: UsdSnapshotState) -> Router {
    Router::new()
        .route("/v1/me/portfolio/snapshots-usd", get(list).post(create))
        .layer(DefaultBodyLimit::max(1_048_576))
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
#[derive(Deserialize)]
struct HistoryQuery {
    limit: Option<String>,
}
fn validation(field: &str, message: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({"error":{"formErrors":[],"fieldErrors":{field:[message]}}})),
    )
        .into_response()
}
fn failure(error: SnapshotError) -> Response {
    match error {
        SnapshotError::InvalidTotal => validation("totalUsd", "Expected a nonnegative total within NUMERIC(18,2)"),
        SnapshotError::InvalidLimit => validation("limit", "Expected an integer between 1 and 100"),
        SnapshotError::Unavailable => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"statusCode":500,"error":"Internal Server Error","message":"Internal Server Error"}))).into_response(),
    }
}
async fn list(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<UsdSnapshotState>,
    query: Result<Query<HistoryQuery>, QueryRejection>,
) -> Response {
    let limit = match query {
        Ok(Query(HistoryQuery { limit: None })) => 30,
        Ok(Query(HistoryQuery { limit: Some(value) })) => match value.trim().parse::<f64>() {
            Ok(number)
                if number.is_finite()
                    && number.fract() == 0.0
                    && (1.0..=100.0).contains(&number) =>
            {
                number as i64
            }
            _ => return failure(SnapshotError::InvalidLimit),
        },
        Err(_) => return failure(SnapshotError::InvalidLimit),
    };
    match state.snapshots.list(&user.id, limit).await {
        Ok(rows) => Json(rows).into_response(),
        Err(error) => failure(error),
    }
}
async fn create(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<UsdSnapshotState>,
    body: Result<Json<Value>, JsonRejection>,
) -> Response {
    let value = match body {
        Ok(Json(value)) => value,
        Err(error) => {
            let status = if error.status() == StatusCode::UNPROCESSABLE_ENTITY {
                StatusCode::BAD_REQUEST
            } else {
                error.status()
            };
            return (status, Json(json!({"statusCode":status.as_u16(),"error":status.canonical_reason(),"message":"Invalid JSON request body"}))).into_response();
        }
    };
    if !value.is_object() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":{"formErrors":["Expected object"],"fieldErrors":{}}})),
        )
            .into_response();
    }
    let Some(Value::Number(total)) = value.get("totalUsd") else {
        return validation("totalUsd", "Expected number");
    };
    match state.snapshots.create(&user.id, &total.to_string()).await {
        Ok(row) => (StatusCode::CREATED, Json(row)).into_response(),
        Err(error) => failure(error),
    }
}
