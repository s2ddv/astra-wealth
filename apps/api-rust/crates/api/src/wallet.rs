//! Authenticated wallet CRUD, mirroring me.routes.ts and toWalletDto.
use crate::auth::{AuthState, AuthenticatedUser};
use application::wallet::WalletService;
use axum::{
    Json, Router,
    extract::{
        DefaultBodyLimit, FromRef, Path, State,
        rejection::{JsonRejection, PathRejection},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get},
};
use chrono::SecondsFormat;
use domain::wallet::{
    Chain, NewWallet, Wallet, WalletAssetSummary, WalletError, WalletRepositoryError,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};
use ts_rs::TS;

#[derive(Clone)]
pub struct WalletState {
    pub auth: AuthState,
    pub wallets: Arc<WalletService>,
}
impl FromRef<WalletState> for AuthState {
    fn from_ref(state: &WalletState) -> Self {
        state.auth.clone()
    }
}
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WalletChain {
    Ethereum,
    Polygon,
    Arbitrum,
    Base,
    Solana,
    Bitcoin,
}
impl From<Chain> for WalletChain {
    fn from(chain: Chain) -> Self {
        match chain {
            Chain::Ethereum => Self::Ethereum,
            Chain::Polygon => Self::Polygon,
            Chain::Arbitrum => Self::Arbitrum,
            Chain::Base => Self::Base,
            Chain::Solana => Self::Solana,
            Chain::Bitcoin => Self::Bitcoin,
        }
    }
}
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WalletDto {
    pub id: String,
    pub address: String,
    pub chain: WalletChain,
    pub nickname: Option<String>,
    pub user_id: String,
    pub assets: Vec<WalletAssetDto>,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WalletAssetDto {
    pub id: String,
    pub symbol: String,
    pub amount: String,
    pub wallet_id: String,
    pub updated_at: String,
}
impl From<Wallet> for WalletDto {
    fn from(wallet: Wallet) -> Self {
        Self {
            id: wallet.id,
            address: wallet.address,
            chain: wallet.chain.into(),
            nickname: wallet.nickname,
            user_id: wallet.user_id,
            assets: wallet.assets.into_iter().map(Into::into).collect(),
            created_at: wallet
                .created_at
                .and_utc()
                .to_rfc3339_opts(SecondsFormat::Millis, true),
            updated_at: wallet
                .updated_at
                .and_utc()
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}
impl From<WalletAssetSummary> for WalletAssetDto {
    fn from(asset: WalletAssetSummary) -> Self {
        Self {
            id: asset.id,
            symbol: asset.symbol,
            amount: asset.amount,
            wallet_id: asset.wallet_id,
            updated_at: asset
                .updated_at
                .and_utc()
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}

pub fn router(state: WalletState) -> Router {
    Router::new()
        .route("/v1/me/wallets", get(list).post(create))
        // PATCH is an explicitly approved extension: the Fastify repository
        // had updateNickname but no corresponding HTTP route.
        .route("/v1/me/wallets/{id}", delete(remove).patch(update_nickname))
        .layer(DefaultBodyLimit::max(1_048_576))
        .with_state(state)
}
// Next: watchlist::router(WatchlistState { auth, ... }) will reuse AuthenticatedUser
// and its own repository/service. Do not add watchlist data to WalletDto.

async fn list(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<WalletState>,
) -> Result<Json<Vec<WalletDto>>, WalletHttpError> {
    Ok(Json(
        state
            .wallets
            .list(&user.id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect(),
    ))
}
async fn create(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<WalletState>,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<(StatusCode, Json<WalletDto>), WalletHttpError> {
    let input = create_payload(json_body(payload)?)?;
    let wallet = state.wallets.create(&user.id, input).await?;
    Ok((StatusCode::CREATED, Json(wallet.into())))
}
async fn remove(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<WalletState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, WalletHttpError> {
    let id = wallet_id(path)?;
    if !state.wallets.remove(&id, &user.id).await? {
        return Err(WalletHttpError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
async fn update_nickname(
    AuthenticatedUser(user): AuthenticatedUser,
    State(state): State<WalletState>,
    path: Result<Path<String>, PathRejection>,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Json<WalletDto>, WalletHttpError> {
    let id = wallet_id(path)?;
    let body = json_body(payload)?;
    let fields = object(&body)?;
    let mut errors = PayloadErrors::default();
    let nickname = string_field(fields.get("nickname"), "nickname", &mut errors);
    if !errors.field_errors.is_empty() {
        return Err(WalletHttpError::Validation(errors));
    }
    let wallet = state
        .wallets
        .update_nickname(&id, &user.id, &nickname)
        .await?
        .ok_or(WalletHttpError::NotFound)?;
    Ok(Json(wallet.into()))
}
fn wallet_id(path: Result<Path<String>, PathRejection>) -> Result<String, WalletHttpError> {
    path.map(|Path(id)| id)
        .ok()
        .filter(|id| !id.is_empty())
        .ok_or(WalletHttpError::InvalidId)
}
fn json_body(body: Result<Json<Value>, JsonRejection>) -> Result<Value, WalletHttpError> {
    body.map(|Json(value)| value)
        .map_err(|rejection| WalletHttpError::Body(rejection.status()))
}
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadErrors {
    form_errors: Vec<String>,
    field_errors: BTreeMap<String, Vec<String>>,
}
impl PayloadErrors {
    fn field(&mut self, name: &str, message: impl Into<String>) {
        self.field_errors
            .entry(name.into())
            .or_default()
            .push(message.into());
    }
    fn merge_report(&mut self, report: &garde::Report) {
        for (path, error) in report.iter() {
            let path = path.to_string();
            // Structural type errors take precedence over value validation.
            self.field_errors
                .entry(path)
                .or_insert_with(|| vec![error.to_string()]);
        }
    }
}
fn object(value: &Value) -> Result<&serde_json::Map<String, Value>, WalletHttpError> {
    value.as_object().ok_or_else(|| {
        WalletHttpError::Validation(PayloadErrors {
            form_errors: vec![format!("Expected object, received {}", json_type(value))],
            ..Default::default()
        })
    })
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
fn string_field(value: Option<&Value>, name: &str, errors: &mut PayloadErrors) -> String {
    match value {
        Some(Value::String(value)) => value.clone(),
        Some(value) => {
            errors.field(
                name,
                format!("Expected string, received {}", json_type(value)),
            );
            String::new()
        }
        None => {
            errors.field(name, "Required");
            String::new()
        }
    }
}
fn create_payload(value: Value) -> Result<NewWallet, WalletHttpError> {
    let fields = object(&value)?;
    let mut errors = PayloadErrors::default();
    let address = string_field(fields.get("address"), "address", &mut errors);
    let nickname = fields
        .get("nickname")
        .map(|value| string_field(Some(value), "nickname", &mut errors));
    let expected = "'ETHEREUM' | 'POLYGON' | 'ARBITRUM' | 'BASE' | 'SOLANA' | 'BITCOIN'";
    let chain = match fields.get("chain") {
        None => Chain::Ethereum,
        Some(Value::String(value)) => value.parse().unwrap_or_else(|_| {
            errors.field(
                "chain",
                format!("Invalid enum value. Expected {expected}, received '{value}'"),
            );
            Chain::Ethereum
        }),
        Some(value) => {
            errors.field(
                "chain",
                format!("Expected {expected}, received {}", json_type(value)),
            );
            Chain::Ethereum
        }
    };
    let input = NewWallet {
        address,
        chain,
        nickname,
    };
    match input.validated() {
        Ok(input) if errors.field_errors.is_empty() => Ok(input),
        Ok(_) => Err(WalletHttpError::Validation(errors)),
        Err(report) => {
            errors.merge_report(&report);
            Err(WalletHttpError::Validation(errors))
        }
    }
}
#[derive(Debug)]
pub enum WalletHttpError {
    Validation(PayloadErrors),
    NotFound,
    InvalidId,
    Repository(WalletRepositoryError),
    Body(StatusCode),
}
impl From<WalletError> for WalletHttpError {
    fn from(error: WalletError) -> Self {
        match error {
            WalletError::Validation(report) => {
                let mut errors = PayloadErrors::default();
                errors.merge_report(&report);
                Self::Validation(errors)
            }
            WalletError::Repository(error) => Self::Repository(error),
        }
    }
}
impl IntoResponse for WalletHttpError {
    fn into_response(self) -> Response {
        match self {
            Self::Validation(errors) => {
                (StatusCode::BAD_REQUEST, Json(json!({"error":errors}))).into_response()
            }
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                Json(json!({"error":"Wallet not found"})),
            )
                .into_response(),
            Self::InvalidId => (
                StatusCode::BAD_REQUEST,
                Json(json!({"error":"Invalid wallet id"})),
            )
                .into_response(),
            Self::Repository(WalletRepositoryError::Conflict) => (
                StatusCode::CONFLICT,
                Json(json!({"error":"Wallet already exists"})),
            )
                .into_response(),
            Self::Repository(error) => {
                tracing::error!(%error, "Wallet operation failed");
                (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"statusCode":500,"error":"Internal Server Error","message":"Internal Server Error"}))).into_response()
            }
            Self::Body(status) => {
                // Fastify uses 400 for malformed JSON, 415 for unsupported media
                // types and 413 for payloads above its default 1 MiB limit.
                let status = if status == StatusCode::UNPROCESSABLE_ENTITY {
                    StatusCode::BAD_REQUEST
                } else {
                    status
                };
                (status, Json(json!({"statusCode":status.as_u16(),"error":status.canonical_reason().unwrap_or("Bad Request"),"message":"Invalid JSON request body"}))).into_response()
            }
        }
    }
}
