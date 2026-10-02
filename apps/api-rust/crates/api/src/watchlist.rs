//! Legacy /v1/me/watchlists contract, including Zod-shaped validation errors.
use crate::auth::{AuthState, AuthenticatedUser};
use application::watchlist::WatchlistService;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, FromRef, Path, State, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use domain::watchlist::*;
use serde_json::{Value, json};
#[derive(Clone)]
pub struct WatchlistState {
    pub auth: AuthState,
    pub watchlists: WatchlistService,
}
impl FromRef<WatchlistState> for AuthState {
    fn from_ref(state: &WatchlistState) -> Self {
        state.auth.clone()
    }
}
pub fn router(state: WatchlistState) -> Router {
    Router::new()
        .route("/v1/me/watchlists", get(list).post(create))
        .route("/v1/me/watchlists/{id}", delete(remove))
        .route("/v1/me/watchlists/{id}/items", post(add_item))
        .route("/v1/me/watchlists/{id}/items/{coin}", delete(remove_item))
        .layer(DefaultBodyLimit::max(1_048_576))
        .with_state(state)
}
struct HttpError(StatusCode, Value);
impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        (self.0, Json(self.1)).into_response()
    }
}
impl From<WatchlistError> for HttpError {
    fn from(error: WatchlistError) -> Self {
        match error {
            WatchlistError::Validation { field, message } => Self(
                StatusCode::BAD_REQUEST,
                json!({"error":{"formErrors":[],"fieldErrors":{field:[message]}}}),
            ),
            WatchlistError::NotFound | WatchlistError::ItemNotFound => {
                Self(StatusCode::NOT_FOUND, json!({"error":error.to_string()}))
            }
            WatchlistError::NameConflict | WatchlistError::ItemConflict => {
                Self(StatusCode::CONFLICT, json!({"error":error.to_string()}))
            }
            WatchlistError::Unavailable(error) => {
                tracing::error!(%error,"Watchlist storage operation failed");
                Self(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({"statusCode":500,"error":"Internal Server Error","message":"Internal Server Error"}),
                )
            }
        }
    }
}
fn json_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
fn field(
    body: Result<Json<Value>, JsonRejection>,
    name: &'static str,
) -> Result<String, HttpError> {
    let Json(value)=body.map_err(|e|{let status=if e.status()==StatusCode::UNPROCESSABLE_ENTITY{StatusCode::BAD_REQUEST}else{e.status()};HttpError(status,json!({"statusCode":status.as_u16(),"error":status.canonical_reason(),"message":"Invalid JSON request body"}))})?;
    let fields=value.as_object().ok_or_else(||HttpError(StatusCode::BAD_REQUEST,json!({"error":{"formErrors":[format!("Expected object, received {}",json_type(&value))],"fieldErrors":{}}})))?;
    match fields.get(name) {
        Some(Value::String(value)) => Ok(value.clone()),
        value => Err(WatchlistError::Validation {
            field: name,
            message: value
                .map(|v| format!("Expected string, received {}", json_type(v)))
                .unwrap_or_else(|| "Required".into()),
        }
        .into()),
    }
}
async fn list(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<WatchlistState>,
) -> Result<Json<Vec<Watchlist>>, HttpError> {
    Ok(Json(state.watchlists.list(&user.id).await?))
}
async fn create(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<WatchlistState>,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<(StatusCode, Json<Watchlist>), HttpError> {
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .watchlists
                .create(&user.id, &field(body, "name")?)
                .await?,
        ),
    ))
}
async fn remove(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<WatchlistState>,
    Path(id): Path<String>,
) -> Result<StatusCode, HttpError> {
    state.watchlists.remove(&user.id, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn add_item(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<WatchlistState>,
    Path(id): Path<String>,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<(StatusCode, Json<WatchlistItem>), HttpError> {
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .watchlists
                .add_item(&user.id, &id, &field(body, "coinId")?)
                .await?,
        ),
    ))
}
async fn remove_item(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<WatchlistState>,
    Path((id, coin)): Path<(String, String)>,
) -> Result<StatusCode, HttpError> {
    state.watchlists.remove_item(&user.id, &id, &coin).await?;
    Ok(StatusCode::NO_CONTENT)
}
