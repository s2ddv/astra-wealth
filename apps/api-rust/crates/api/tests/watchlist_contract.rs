use application::{UserService, watchlist::WatchlistService};
use astra_api::{
    auth::AuthState,
    watchlist::{WatchlistState, router},
};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use domain::{
    AuthIdentity, NewUser, RepositoryFuture, User, UserRepository, UserUpdate, watchlist::*,
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

struct Lists;
fn list() -> Watchlist {
    Watchlist {
        id: "owned".into(),
        name: "Favorites".into(),
        user_id: "local-owner".into(),
        items: vec![],
        created_at: "2020-01-01T00:00:00.000Z".into(),
        updated_at: "2020-01-01T00:00:00.000Z".into(),
    }
}
impl WatchlistRepository for Lists {
    fn list<'a>(&'a self, owner: &'a str) -> WatchlistFuture<'a, Vec<Watchlist>> {
        assert_eq!(owner, "local-owner");
        Box::pin(async { Ok(vec![list()]) })
    }
    fn create<'a>(&'a self, owner: &'a str, name: String) -> WatchlistFuture<'a, Watchlist> {
        assert_eq!(owner, "local-owner");
        Box::pin(async move {
            if name == "duplicate" {
                Err(WatchlistError::NameConflict)
            } else {
                assert_eq!(name, "Favorites");
                Ok(list())
            }
        })
    }
    fn remove<'a>(&'a self, owner: &'a str, id: &'a str) -> WatchlistFuture<'a, ()> {
        assert_eq!(owner, "local-owner");
        Box::pin(async move {
            if id == "owned" {
                Ok(())
            } else {
                Err(WatchlistError::NotFound)
            }
        })
    }
    fn add_item<'a>(
        &'a self,
        owner: &'a str,
        id: &'a str,
        coin: String,
    ) -> WatchlistFuture<'a, WatchlistItem> {
        assert_eq!(owner, "local-owner");
        Box::pin(async move {
            if id != "owned" {
                return Err(WatchlistError::NotFound);
            }
            if coin == "duplicate" {
                return Err(WatchlistError::ItemConflict);
            }
            assert_eq!(coin, "bitcoin");
            Ok(WatchlistItem {
                id: "item".into(),
                coin_id: coin,
                added_at: "2020-01-01T00:00:00.000Z".into(),
            })
        })
    }
    fn remove_item<'a>(
        &'a self,
        owner: &'a str,
        id: &'a str,
        coin: &'a str,
    ) -> WatchlistFuture<'a, ()> {
        assert_eq!(owner, "local-owner");
        Box::pin(async move {
            if id == "owned" && coin == "bitcoin" {
                Ok(())
            } else {
                Err(WatchlistError::ItemNotFound)
            }
        })
    }
}
async fn request(method: &str, path: &str, body: Value, auth: bool) -> (StatusCode, Value) {
    let app = router(WatchlistState {
        auth: AuthState {
            users: UserService::new(Arc::new(Users)),
            verifier: None,
            production: false,
            dev_user_id: auth.then(|| "local-owner".into()),
        },
        watchlists: WatchlistService::new(Arc::new(Lists)),
    });
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1_048_576).await.unwrap();
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
async fn legacy_success_contract_and_errors() {
    assert_eq!(
        request("GET", "/v1/me/watchlists", Value::Null, false)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request("GET", "/v1/me/watchlists", Value::Null, true).await,
        (StatusCode::OK, json!([list()]))
    );
    assert_eq!(
        request(
            "POST",
            "/v1/me/watchlists",
            json!({"name":" Favorites ","userId":"forged"}),
            true
        )
        .await,
        (StatusCode::CREATED, json!(list()))
    );
    assert_eq!(
        request(
            "POST",
            "/v1/me/watchlists",
            json!({"name":"duplicate"}),
            true
        )
        .await,
        (
            StatusCode::CONFLICT,
            json!({"error":"Watchlist name already exists"})
        )
    );
    assert_eq!(
        request(
            "POST",
            "/v1/me/watchlists/owned/items",
            json!({"coinId":" bitcoin "}),
            true
        )
        .await
        .0,
        StatusCode::CREATED
    );
    assert_eq!(
        request(
            "POST",
            "/v1/me/watchlists/owned/items",
            json!({"coinId":"duplicate"}),
            true
        )
        .await,
        (
            StatusCode::CONFLICT,
            json!({"error":"Coin already in watchlist"})
        )
    );
    assert_eq!(
        request(
            "POST",
            "/v1/me/watchlists/foreign/items",
            json!({"coinId":"bitcoin"}),
            true
        )
        .await,
        (
            StatusCode::NOT_FOUND,
            json!({"error":"Watchlist not found"})
        )
    );
    assert_eq!(
        request(
            "DELETE",
            "/v1/me/watchlists/owned/items/bitcoin",
            Value::Null,
            true
        )
        .await,
        (StatusCode::NO_CONTENT, Value::Null)
    );
    assert_eq!(
        request(
            "DELETE",
            "/v1/me/watchlists/foreign/items/bitcoin",
            Value::Null,
            true
        )
        .await,
        (
            StatusCode::NOT_FOUND,
            json!({"error":"Watchlist or item not found"})
        )
    );
    assert_eq!(
        request("DELETE", "/v1/me/watchlists/owned", Value::Null, true)
            .await
            .0,
        StatusCode::NO_CONTENT
    );
}
#[tokio::test]
async fn zod_flattened_errors_and_utf16_limits() {
    for (payload, message) in [
        (json!({}), "Required"),
        (json!({"name":1}), "Expected string, received number"),
        (
            json!({"name":" "}),
            "String must contain at least 1 character(s)",
        ),
        (
            json!({"name":"x".repeat(65)}),
            "String must contain at most 64 character(s)",
        ),
        (
            json!({"name":"😀".repeat(33)}),
            "String must contain at most 64 character(s)",
        ),
    ] {
        assert_eq!(
            request("POST", "/v1/me/watchlists", payload, true).await,
            (
                StatusCode::BAD_REQUEST,
                json!({"error":{"formErrors":[],"fieldErrors":{"name":[message]}}})
            )
        );
    }
}
