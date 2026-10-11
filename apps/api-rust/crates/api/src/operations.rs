use crate::{
    accounts::AccountHttpError,
    auth::{AuthState, AuthenticatedUser},
};
use application::operations::{NewOperation, Operation, OperationLedger, OperationService};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, FromRef, Path, State, rejection::JsonRejection},
    http::StatusCode,
    routing::get,
};
use domain::accounts::AccountError;
#[derive(Clone)]
pub struct OperationState {
    pub auth: AuthState,
    pub operations: OperationService,
}
impl FromRef<OperationState> for AuthState {
    fn from_ref(s: &OperationState) -> Self {
        s.auth.clone()
    }
}
pub fn router(state: OperationState) -> Router {
    Router::new()
        .route("/v1/accounts/{id}/operations", get(list).post(append))
        .layer(DefaultBodyLimit::max(16_384))
        .with_state(state)
}
async fn list(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<OperationState>,
    Path(id): Path<String>,
) -> Result<Json<OperationLedger>, AccountHttpError> {
    Ok(Json(state.operations.list(&user.id, &id).await?))
}
async fn append(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<OperationState>,
    Path(id): Path<String>,
    input: Result<Json<NewOperation>, JsonRejection>,
) -> Result<(StatusCode, Json<Operation>), AccountHttpError> {
    let Json(input) = input.map_err(|_| AccountError::Validation("Corpo JSON inválido"))?;
    Ok((
        StatusCode::CREATED,
        Json(state.operations.append(&user.id, &id, input).await?),
    ))
}
