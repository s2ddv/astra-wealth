use application::{UserService, operations::*};
use astra_api::{
    auth::AuthState,
    operations::{OperationState, router},
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use domain::{
    AuthIdentity, NewUser, RepositoryFuture, User, UserRepository, UserUpdate, accounts::*,
};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;
struct Users;
impl UserRepository for Users {
    fn find_by_id<'a>(&'a self, id: &'a str) -> RepositoryFuture<'a, Option<User>> {
        Box::pin(async move {
            Ok((id == "local-owner").then(|| User {
                id: id.into(),
                auth_id: "different-auth-id".into(),
                email: "test@example.com".into(),
                name: None,
                created_at: time(),
                updated_at: time(),
            }))
        })
    }
    fn find_by_email<'a>(&'a self, _: &'a str) -> RepositoryFuture<'a, Option<User>> {
        unreachable!()
    }
    fn create(&self, _: NewUser) -> RepositoryFuture<'_, User> {
        unreachable!()
    }
    fn update<'a>(&'a self, _: &'a str, _: UserUpdate) -> RepositoryFuture<'a, Option<User>> {
        unreachable!()
    }
    fn upsert_identity(&self, _: AuthIdentity) -> RepositoryFuture<'_, User> {
        unreachable!()
    }
}
fn time() -> chrono::NaiveDateTime {
    chrono::DateTime::from_timestamp_millis(1_700_000_000_123)
        .unwrap()
        .naive_utc()
}

struct Operations;
impl OperationRepository for Operations {
    fn list<'a>(&'a self, owner: &'a str, account: &'a str) -> AccountFuture<'a, Vec<Operation>> {
        assert_eq!(owner, "local-owner");
        Box::pin(async move {
            if account == "owned" {
                Ok(vec![])
            } else {
                Err(AccountError::NotFound)
            }
        })
    }
    fn append<'a>(
        &'a self,
        owner: &'a str,
        _: &'a str,
        input: NewOperation,
    ) -> AccountFuture<'a, Operation> {
        assert_eq!(owner, "local-owner");
        input.validate().unwrap();
        Box::pin(async { Err(AccountError::Conflict) })
    }
}
async fn request(
    method: &str,
    path: &str,
    payload: Value,
    authenticated: bool,
) -> (StatusCode, Value) {
    let app = router(OperationState {
        auth: AuthState {
            users: UserService::new(Arc::new(Users)),
            verifier: None,
            allow_dev_auth: true,
            dev_user_id: authenticated.then(|| "local-owner".into()),
        },
        operations: OperationService::new(Arc::new(Operations)),
    });
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1_048_576).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
#[tokio::test]
async fn operations_require_local_identity_and_strict_inputs() {
    for method in ["GET", "POST"] {
        assert_eq!(
            request(method, "/v1/accounts/owned/operations", json!(null), false)
                .await
                .0,
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        request("GET", "/v1/accounts/foreign/operations", json!(null), true)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request("GET", "/v1/accounts/owned/operations", json!(null), true)
            .await
            .1,
        json!({"operations":[],"positions":[],"results":[]})
    );
    let valid = json!({"requestId":"request","kind":"BUY","holdingId":"holding","market":"BR","quantity":"1","amount":"10","fees":"0","currency":"BRL","occurredAt":"2020-01-01T00:00:00Z"});
    assert_eq!(
        request("POST", "/v1/accounts/owned/operations", valid.clone(), true)
            .await
            .0,
        StatusCode::CONFLICT
    );
    for (field, value) in [
        ("userId", json!("foreign")),
        ("accountId", json!("foreign")),
        ("origin", json!("IMPORTED")),
        ("quantity", json!(1)),
        ("amount", json!("-1")),
        ("kind", json!("DEPOSIT")),
        ("occurredAt", json!("2999-01-01T00:00:00Z")),
    ] {
        let mut bad = valid.clone();
        bad[field] = value;
        assert_eq!(
            request("POST", "/v1/accounts/owned/operations", bad, true)
                .await
                .0,
            StatusCode::BAD_REQUEST,
            "{field}"
        );
    }
}
