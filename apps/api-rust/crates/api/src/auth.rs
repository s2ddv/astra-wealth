//! Shared extractor for all future protected handlers.
use application::UserService;
use axum::{
    Json,
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use domain::User;
use infrastructure::auth::{AuthError, SupabaseAuth};
use serde::Serialize;
use std::sync::Arc;

#[derive(Clone)]
pub struct AuthState {
    pub users: UserService,
    pub verifier: Option<Arc<SupabaseAuth>>,
    pub dev_user_id: Option<String>,
    pub allow_dev_auth: bool,
}
#[derive(Clone, Debug)]
pub struct AuthenticatedUser(pub User);
#[derive(Debug)]
pub struct AuthRejection(pub &'static str);
#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<&'static str>,
}
impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorBody {
                error: self.0,
                hint: (self.0 == "Authentication required").then_some(
                    "Use Bearer token; local development requires NODE_ENV=development and ALLOW_DEV_AUTH=true",
                ),
            }),
        )
            .into_response()
    }
}
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
    AuthState: FromRef<S>,
{
    type Rejection = AuthRejection;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = AuthState::from_ref(state);
        // Supplied credentials take precedence over every development shortcut.
        let authorization = parts.headers.get(axum::http::header::AUTHORIZATION);
        if authorization.is_none() {
            if state.allow_dev_auth {
                let header = if state.verifier.is_none() {
                    parts.headers.get("x-user-id")
                        .map(|value| value.to_str().map_err(|_| AuthRejection("Invalid X-User-Id header")))
                        .transpose()?.filter(|id| !id.is_empty())
                } else { None };
                if let Some(id) = header.or(state.dev_user_id.as_deref()) {
                    let user = state.users.find_by_id(id).await
                        .map_err(|_| AuthRejection("Authentication failed"))?
                        .ok_or(AuthRejection("Invalid X-User-Id header"))?;
                    let user = Self(user);
                    parts.extensions.insert(user.clone());
                    return Ok(user);
                }
            }
            return Err(AuthRejection(if state.allow_dev_auth && state.verifier.is_none() {
                "Authentication required"
            } else { "Missing or invalid authorization header" }));
        }
        // Preserve the legacy Bearer/raw-JWT contract, but never treat a supplied
        // empty header as permission to impersonate the development user.
        let token = authorization.unwrap().to_str()
            .map_err(|_| AuthRejection("Invalid or expired token"))?
            .replacen("Bearer ", "", 1);
        if token.is_empty() { return Err(AuthRejection("Invalid or expired token")); }
        let verifier = state.verifier.as_ref()
            .ok_or(AuthRejection("Authentication failed"))?;
        let identity = verifier.verify(&token).await.map_err(|error| match error {
            AuthError::InvalidToken => AuthRejection("Invalid or expired token"),
            AuthError::Unavailable => AuthRejection("Authentication failed"),
        })?;
        let user = state.users.authenticate(identity).await.map_err(|error| {
            tracing::error!(%error, "User synchronization failed");
            AuthRejection("Authentication failed")
        })?;
        let user = Self(user);
        parts.extensions.insert(user.clone());
        Ok(user)
    }
}
// Wallet handlers use `AuthenticatedUser(user): AuthenticatedUser` and pass
// user.id to WalletService. Do not authorize wallet access using user.auth_id.

/// Development impersonation requires both explicit settings; missing or
/// misspelled NODE_ENV values keep real authentication mandatory.
pub fn development_auth_enabled(node_env: Option<&str>, opt_in: Option<&str>) -> bool {
    node_env == Some("development") && opt_in == Some("true")
}

#[cfg(test)]
mod tests {
    use super::development_auth_enabled;
    #[test]
    fn development_auth_requires_both_explicit_settings() {
        for environment in [None, Some("production"), Some("test"), Some("staging"), Some("Development"), Some("development")] {
            for flag in [None, Some("false"), Some("1"), Some("TRUE"), Some("true")] {
                assert_eq!(development_auth_enabled(environment, flag), environment == Some("development") && flag == Some("true"));
            }
        }
    }
}
