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
    pub production: bool,
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
                    "Use Bearer token in Authorization header or set DEV_USER_ID in .env",
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
        // With Supabase configured, the auth plugin runs before requireUserId.
        // Without it, match the legacy X-User-Id fallback in development only.
        let legacy = state.verifier.is_none() && !state.production;
        if legacy
            && let Some(id) = parts
                .headers
                .get("x-user-id")
                .and_then(|value| value.to_str().ok())
                .filter(|value| !value.is_empty())
        {
            let user = state
                .users
                .find_by_id(id)
                .await
                .map_err(|_| AuthRejection("Authentication failed"))?
                .ok_or(AuthRejection("Invalid X-User-Id header"))?;
            let user = Self(user);
            parts.extensions.insert(user.clone());
            return Ok(user);
        }
        // Fastify uses a case-sensitive replace, also accepting a raw JWT.
        let token = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .map(|value| value.to_str().map(|value| value.replacen("Bearer ", "", 1)))
            .transpose()
            .map_err(|_| AuthRejection("Invalid or expired token"))?;
        if legacy || token.as_ref().is_none_or(String::is_empty) {
            if !state.production
                && let Some(id) = &state.dev_user_id
            {
                match state.users.find_by_id(id).await {
                    Ok(Some(user)) => {
                        let user = Self(user);
                        parts.extensions.insert(user.clone());
                        return Ok(user);
                    }
                    Err(error) => {
                        tracing::error!(%error, "Development authentication failed");
                    }
                    Ok(None) => {}
                }
            }
            return Err(AuthRejection(if legacy {
                "Authentication required"
            } else {
                "Missing or invalid authorization header"
            }));
        }
        let token = token.ok_or(AuthRejection("Missing or invalid authorization header"))?;
        let verifier = state
            .verifier
            .as_ref()
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
