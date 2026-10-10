use application::{
    UserService,
    fx::{FxFuture, FxProvider, FxService},
    portfolio::*,
};
use astra_api::{
    auth::AuthState,
    portfolio::{PortfolioState, router},
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use domain::{
    AuthIdentity, NewUser, RepositoryFuture, User, UserRepository, UserUpdate,
    asset_icon::{AssetCache, IconFuture},
    wallet::Chain,
};
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

struct Cache;
impl AssetCache for Cache {
    fn get<'a>(&'a self, _: &'a str) -> IconFuture<'a, Option<String>> {
        Box::pin(async { Ok(None) })
    }
    fn set<'a>(&'a self, _: &'a str, _: &'a str, _: u64) -> IconFuture<'a, ()> {
        Box::pin(async { Ok(()) })
    }
}
struct Fx;
impl FxProvider for Fx {
    fn closing_on_or_before(&self, _: chrono::NaiveDate) -> FxFuture<'_> {
        Box::pin(async { Ok(None) })
    }
}
struct Prices;
impl BrlPriceProvider for Prices {
    fn price<'a>(&'a self, _: Chain, _: Option<&'a str>) -> PortfolioFuture<'a, Option<BrlQuote>> {
        unreachable!()
    }
}
struct Repo;
impl PortfolioRepository for Repo {
    fn load<'a>(&'a self, owner: &'a str) -> PortfolioFuture<'a, PortfolioInput> {
        assert_eq!(owner, "local-owner");
        Box::pin(async {
            Ok(PortfolioInput {
                observed_at: chrono::Utc::now().to_rfc3339(),
                positions: vec![],
                contributions: vec![],
            })
        })
    }
    fn save<'a>(&'a self, owner: &'a str, _: &'a PortfolioSnapshot) -> PortfolioFuture<'a, String> {
        assert_eq!(owner, "local-owner");
        Box::pin(async { Ok("saved".into()) })
    }
    fn history<'a>(&'a self, owner: &'a str) -> PortfolioFuture<'a, Vec<SavedSnapshot>> {
        assert_eq!(owner, "local-owner");
        Box::pin(async { Ok(vec![]) })
    }
}
fn app() -> axum::Router {
    let fx = FxService::new(Arc::new(Fx), Arc::new(Cache));
    router(PortfolioState {
        auth: AuthState {
            users: UserService::new(Arc::new(Users)),
            verifier: None,
            dev_user_id: None,
            allow_dev_auth: true,
        },
        portfolio: PortfolioService::new(Arc::new(Repo), fx.clone(), Arc::new(Prices)),
        fx,
    })
}
#[tokio::test]
async fn authentication_local_owner_decimal_contract_and_no_shared_cache() {
    for path in [
        "/v1/me/portfolio",
        "/v1/me/portfolio/snapshots",
        "/v1/me/fx/usd-brl",
    ] {
        let response = app()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    let response = app()
        .oneshot(
            Request::builder()
                .uri("/v1/me/portfolio")
                .header("x-user-id", "local-owner")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let value: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 100000).await.unwrap()).unwrap();
    assert_eq!(value["knownValueBrl"], "0");
    assert_eq!(value["complete"], true);
    assert!(value.get("userId").is_none());
}
#[tokio::test]
async fn historical_dates_and_snapshot_methods() {
    for (path, status) in [
        ("/v1/me/fx/usd-brl/2024-01-07", StatusCode::OK),
        ("/v1/me/fx/usd-brl/2999-01-01", StatusCode::BAD_REQUEST),
        ("/v1/me/fx/usd-brl/2024-02-30", StatusCode::BAD_REQUEST),
    ] {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header("x-user-id", "local-owner")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
    let response = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/me/portfolio/snapshots")
                .header("x-user-id", "local-owner")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let response = app()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/v1/me/portfolio/snapshots")
                .header("x-user-id", "local-owner")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}
