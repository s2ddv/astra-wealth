use application::{UserService, accounts::AccountService};
use astra_api::{
    accounts::{AccountState, router},
    auth::AuthState,
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

struct Accounts;
impl AccountRepository for Accounts {
    fn list<'a>(&'a self, owner: &'a str) -> AccountFuture<'a, Vec<Account>> {
        assert_eq!(owner, "local-owner");
        Box::pin(async { Ok(vec![]) })
    }
    fn find<'a>(&'a self, owner: &'a str, _: &'a str) -> AccountFuture<'a, AccountDetail> {
        assert_eq!(owner, "local-owner");
        Box::pin(async { Err(AccountError::NotFound) })
    }
    fn create<'a>(&'a self, owner: &'a str, input: NewAccount) -> AccountFuture<'a, Account> {
        assert_eq!(owner, "local-owner");
        input.validate().unwrap();
        Box::pin(async { Err(AccountError::Conflict) })
    }
    fn remove<'a>(&'a self, _: &'a str, _: &'a str) -> AccountFuture<'a, ()> {
        unreachable!()
    }
    fn save_holding<'a>(
        &'a self,
        _: &'a str,
        _: &'a str,
        _: Option<&'a str>,
        _: NewHolding,
    ) -> AccountFuture<'a, Holding> {
        unreachable!()
    }
    fn add_contribution<'a>(
        &'a self,
        _: &'a str,
        _: &'a str,
        _: NewContribution,
    ) -> AccountFuture<'a, Contribution> {
        unreachable!()
    }
    fn remove_record<'a>(
        &'a self,
        _: &'a str,
        _: &'a str,
        _: &'a str,
        _: bool,
    ) -> AccountFuture<'a, ()> {
        unreachable!()
    }
}
async fn request(
    method: &str,
    path: &str,
    payload: Value,
    authenticated: bool,
) -> (StatusCode, Value) {
    let app = router(AccountState {
        auth: AuthState {
            users: UserService::new(Arc::new(Users)),
            verifier: None,
            production: false,
            dev_user_id: authenticated.then(|| "local-owner".into()),
        },
        accounts: AccountService::new(Arc::new(Accounts)),
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
async fn authentication_ownership_and_errors() {
    assert_eq!(
        request("GET", "/v1/accounts", json!(null), false).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request("GET", "/v1/accounts", json!(null), true).await,
        (StatusCode::OK, json!([]))
    );
    assert_eq!(
        request("GET", "/v1/accounts/foreign", json!(null), true)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(
            "POST",
            "/v1/accounts",
            json!({"name":"Saved","kind":"MANUAL","baseCurrency":"BRL"}),
            true
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
}
#[tokio::test]
async fn rejects_forged_ownership_origin_numeric_json_and_invalid_fx() {
    for payload in [
        json!({"name":"Saved","kind":"MANUAL","baseCurrency":"BRL","userId":"foreign"}),
        json!({"name":"Saved","kind":"WALLET","baseCurrency":"USD"}),
    ] {
        assert_eq!(
            request("POST", "/v1/accounts", payload, true).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    for payload in [
        json!({"kind":"DEPOSIT","amount":1,"currency":"BRL","occurredAt":"2020-01-01T00:00:00Z"}),
        json!({"kind":"DEPOSIT","amount":"1","currency":"USD","occurredAt":"2020-01-01T00:00:00Z"}),
        json!({"kind":"DEPOSIT","amount":"1","currency":"BRL","occurredAt":"2020-01-01T00:00:00Z","origin":"IMPORTED"}),
    ] {
        assert_eq!(
            request("POST", "/v1/accounts/owned/contributions", payload, true)
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
}
