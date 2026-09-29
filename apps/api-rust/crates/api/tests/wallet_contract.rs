use application::{UserService, wallet::WalletService};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use domain::{
    AuthIdentity, NewUser, RepositoryFuture, User, UserRepository, UserUpdate, wallet::*,
};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;
use zora_api::{
    auth::AuthState,
    wallet::{WalletState, router},
};

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
fn wallet() -> Wallet {
    Wallet {
        id: "owned".into(),
        address: "0xde709f2102306220921060314715629080e2fb77".into(),
        chain: Chain::Ethereum,
        nickname: None,
        user_id: "local-owner".into(),
        created_at: time(),
        updated_at: time(),
        last_synced_at: None,
        assets: vec![WalletAssetSummary {
            id: "asset".into(),
            symbol: "ETH".into(),
            amount: "1.23456789123456789".into(),
            wallet_id: "owned".into(),
            updated_at: time(),
        }],
    }
}
// A strict boundary spy: invalid input must never reach persistence, and every
// successful call must carry the local user id, not the authentication subject.
struct Wallets;
impl WalletRepository for Wallets {
    fn create<'a>(&'a self, owner: &'a str, input: NewWallet) -> WalletFuture<'a, Wallet> {
        assert_eq!(owner, "local-owner");
        assert!(input.clone().validated().is_ok());
        Box::pin(async move {
            if input.nickname.as_deref() == Some("duplicate") {
                Err(WalletRepositoryError::Conflict)
            } else {
                Ok(wallet())
            }
        })
    }
    fn find_by_id<'a>(&'a self, id: &'a str, owner: &'a str) -> WalletFuture<'a, Option<Wallet>> {
        assert_eq!(owner, "local-owner");
        Box::pin(async move {
            Ok((id == "owned").then(|| {
                let mut w = wallet();
                w.nickname = Some("Savings".into());
                w
            }))
        })
    }
    fn find_by_user_id<'a>(&'a self, owner: &'a str) -> WalletFuture<'a, Vec<Wallet>> {
        assert_eq!(owner, "local-owner");
        Box::pin(async { Ok(vec![wallet()]) })
    }
    fn update_nickname<'a>(
        &'a self,
        id: &'a str,
        owner: &'a str,
        nickname: &'a str,
    ) -> WalletFuture<'a, u64> {
        assert_eq!(owner, "local-owner");
        assert_eq!(nickname, "Savings");
        Box::pin(async move { Ok(u64::from(id == "owned")) })
    }
    fn delete<'a>(&'a self, id: &'a str, owner: &'a str) -> WalletFuture<'a, u64> {
        assert_eq!(owner, "local-owner");
        Box::pin(async move { Ok(u64::from(id == "owned")) })
    }
}
fn app() -> Router {
    router(WalletState {
        auth: AuthState {
            users: UserService::new(Arc::new(Users)),
            verifier: None,
            production: false,
            dev_user_id: None,
        },
        wallets: Arc::new(WalletService::new(Arc::new(Wallets))),
    })
}
async fn request(method: &str, path: &str, body: Value, auth: bool) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if auth {
        req = req.header("x-user-id", "local-owner");
    }
    let res = app()
        .oneshot(req.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), 2_000_000).await.unwrap();
    (
        status,
        if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        },
    )
}
#[tokio::test]
async fn authenticated_crud_preserves_dto_and_owner_contract() {
    let path = "/v1/me/wallets";
    assert_eq!(
        request("GET", path, Value::Null, false).await.0,
        StatusCode::UNAUTHORIZED
    );
    let (status, body) = request("GET", path, Value::Null, true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body[0]["userId"], "local-owner");
    assert_eq!(body[0]["assets"][0]["amount"], "1.23456789123456789");
    assert_eq!(body[0]["createdAt"], "2023-11-14T22:13:20.123Z");
    assert_eq!(body[0].as_object().unwrap().len(), 8);
    let payload = json!({"address":wallet().address,"userId":"attacker"});
    assert_eq!(
        request("POST", path, payload.clone(), true).await.0,
        StatusCode::CREATED
    );
    let mut duplicate = payload;
    duplicate["nickname"] = json!("duplicate");
    assert_eq!(
        request("POST", path, duplicate, true).await,
        (
            StatusCode::CONFLICT,
            json!({"error":"Wallet already exists"})
        )
    );
    let (status, updated) = request(
        "PATCH",
        "/v1/me/wallets/owned",
        json!({"nickname":" Savings "}),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["nickname"], "Savings");
    for method in ["PATCH", "DELETE"] {
        assert_eq!(
            request(
                method,
                "/v1/me/wallets/foreign",
                json!({"nickname":"Savings"}),
                true
            )
            .await,
            (StatusCode::NOT_FOUND, json!({"error":"Wallet not found"}))
        );
    }
    assert_eq!(
        request("DELETE", "/v1/me/wallets/owned", Value::Null, true)
            .await
            .0,
        StatusCode::NO_CONTENT
    );
}
#[tokio::test]
async fn rejects_invalid_payloads_before_repository_calls() {
    for (payload, field) in [
        (json!({}), "address"),
        (json!({"address":"invalid"}), "address"),
        (
            json!({"address":wallet().address,"chain":"INVALID"}),
            "chain",
        ),
        (
            json!({"address":wallet().address,"nickname":null}),
            "nickname",
        ),
    ] {
        let (status, body) = request("POST", "/v1/me/wallets", payload, true).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"]["fieldErrors"][field].is_array());
        assert_eq!(body["error"]["formErrors"], json!([]));
    }
    for payload in [
        json!({}),
        json!({"nickname":" "}),
        json!({"nickname":null}),
        json!({"nickname":"x".repeat(65)}),
    ] {
        assert_eq!(
            request("PATCH", "/v1/me/wallets/owned", payload, true)
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
}
