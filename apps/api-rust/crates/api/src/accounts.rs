use crate::auth::{AuthState, AuthenticatedUser};
use application::accounts::AccountService;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, FromRef, Path, State, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use domain::accounts::*;
use serde_json::json;

#[derive(Clone)]
pub struct AccountState {
    pub auth: AuthState,
    pub accounts: AccountService,
}
impl FromRef<AccountState> for AuthState {
    fn from_ref(state: &AccountState) -> Self {
        state.auth.clone()
    }
}
pub fn router(state: AccountState) -> Router {
    Router::new()
        .route("/v1/accounts", get(list).post(create))
        .route("/v1/accounts/{id}", get(find).delete(remove))
        .route("/v1/accounts/{id}/holdings", post(add_holding))
        .route(
            "/v1/accounts/{id}/holdings/{record}",
            delete(remove_holding).put(update_holding),
        )
        .route("/v1/accounts/{id}/contributions", post(add_contribution))
        .route(
            "/v1/accounts/{id}/contributions/{record}",
            delete(remove_contribution),
        )
        .layer(DefaultBodyLimit::max(1_048_576))
        .with_state(state)
}
pub struct AccountHttpError(AccountError);
impl From<AccountError> for AccountHttpError {
    fn from(error: AccountError) -> Self {
        Self(error)
    }
}
impl IntoResponse for AccountHttpError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self.0 {
            AccountError::Validation(message) => {
                (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", message)
            }
            AccountError::NotFound => (
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                "Account or record not found",
            ),
            AccountError::Conflict => (StatusCode::CONFLICT, "CONFLICT", "Record already exists"),
            AccountError::Unavailable(error) => {
                tracing::error!(%error,"Account storage operation failed");
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "STORAGE_UNAVAILABLE",
                    "Account storage unavailable",
                )
            }
        };
        (status, Json(json!({"code":code,"error":message}))).into_response()
    }
}
fn body<T>(value: Result<Json<T>, JsonRejection>) -> Result<T, AccountHttpError> {
    value
        .map(|Json(v)| v)
        .map_err(|_| AccountError::Validation("Invalid JSON request body").into())
}
async fn list(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AccountState>,
) -> Result<Json<Vec<Account>>, AccountHttpError> {
    Ok(Json(state.accounts.list(&user.id).await?))
}
async fn find(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AccountState>,
    Path(id): Path<String>,
) -> Result<Json<AccountDetail>, AccountHttpError> {
    Ok(Json(state.accounts.find(&user.id, &id).await?))
}
async fn create(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AccountState>,
    input: Result<Json<NewAccount>, JsonRejection>,
) -> Result<(StatusCode, Json<Account>), AccountHttpError> {
    Ok((
        StatusCode::CREATED,
        Json(state.accounts.create(&user.id, body(input)?).await?),
    ))
}
async fn remove(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AccountState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AccountHttpError> {
    state.accounts.remove(&user.id, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn add_holding(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AccountState>,
    Path(id): Path<String>,
    input: Result<Json<NewHolding>, JsonRejection>,
) -> Result<(StatusCode, Json<Holding>), AccountHttpError> {
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .accounts
                .save_holding(&user.id, &id, None, body(input)?)
                .await?,
        ),
    ))
}
async fn update_holding(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AccountState>,
    Path((id, record)): Path<(String, String)>,
    input: Result<Json<NewHolding>, JsonRejection>,
) -> Result<Json<Holding>, AccountHttpError> {
    Ok(Json(
        state
            .accounts
            .save_holding(&user.id, &id, Some(&record), body(input)?)
            .await?,
    ))
}
async fn add_contribution(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AccountState>,
    Path(id): Path<String>,
    input: Result<Json<NewContribution>, JsonRejection>,
) -> Result<(StatusCode, Json<Contribution>), AccountHttpError> {
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .accounts
                .add_contribution(&user.id, &id, body(input)?)
                .await?,
        ),
    ))
}
async fn remove_holding(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AccountState>,
    Path((id, record)): Path<(String, String)>,
) -> Result<StatusCode, AccountHttpError> {
    state
        .accounts
        .remove_record(&user.id, &id, &record, true)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn remove_contribution(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<AccountState>,
    Path((id, record)): Path<(String, String)>,
) -> Result<StatusCode, AccountHttpError> {
    state
        .accounts
        .remove_record(&user.id, &id, &record, false)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
