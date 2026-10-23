use application::{UserService, usd_snapshots::*};
use astra_api::{
    auth::AuthState,
    usd_snapshots::{UsdSnapshotState, router},
};
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use domain::{AuthIdentity, NewUser, RepositoryFuture, User, UserRepository, UserUpdate};
use serde_json::Value;
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

struct Snapshots;
fn row(total: &str) -> UsdSnapshot {
    UsdSnapshot {
        id: "usd-only".into(),
        user_id: "local-owner".into(),
        total_usd: total.into(),
        created_at: "2026-01-01T00:00:00.000Z".into(),
    }
}
impl UsdSnapshotRepository for Snapshots {
    fn list<'a>(&'a self, owner: &'a str, limit: i64) -> SnapshotFuture<'a, Vec<UsdSnapshot>> {
        assert_eq!(owner, "local-owner");
        Box::pin(async move { Ok(vec![row(&limit.to_string())]) })
    }
    fn create<'a>(&'a self, owner: &'a str, total: String) -> SnapshotFuture<'a, UsdSnapshot> {
        assert_eq!(owner, "local-owner");
        Box::pin(async move { Ok(row(&total)) })
    }
}
async fn request(method: &str, path: &str, body: &str, auth: bool) -> (u16, Value) {
    let app = router(UsdSnapshotState {
        auth: AuthState {
            users: UserService::new(Arc::new(Users)),
            verifier: None,
            allow_dev_auth: true,
            dev_user_id: auth.then(|| "local-owner".into()),
        },
        snapshots: UsdSnapshotService::new(Arc::new(Snapshots)),
    });
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    (
        status,
        serde_json::from_slice(&to_bytes(response.into_body(), 2_000_000).await.unwrap()).unwrap(),
    )
}
const PATH: &str = "/v1/me/portfolio/snapshots-usd";
#[tokio::test]
async fn usd_contract_owner_default_limit_and_exact_rounding() {
    assert_eq!(request("GET", PATH, "", false).await.0, 401);
    assert_eq!(
        request("POST", PATH, r#"{"totalUsd":1}"#, false).await.0,
        401
    );
    let (status, rows) = request("GET", PATH, "", true).await;
    assert_eq!(status, 200);
    assert_eq!(rows[0]["totalUsd"], "30");
    assert_eq!(
        request("GET", &format!("{PATH}?limit=1e2"), "", true)
            .await
            .1[0]["totalUsd"],
        "100"
    );
    for (total, expected) in [
        ("0", "0.00"),
        ("123.455", "123.46"),
        ("9999999999999999.99", "9999999999999999.99"),
    ] {
        let (status, row) = request(
            "POST",
            PATH,
            &format!(r#"{{"totalUsd":{total},"userId":"forged"}}"#),
            true,
        )
        .await;
        assert_eq!(status, 201);
        assert_eq!(row["totalUsd"], expected);
        assert_eq!(row["userId"], "local-owner");
        assert_eq!(row.as_object().unwrap().len(), 4);
    }
}
#[tokio::test]
async fn invalid_inputs_never_reach_storage_and_return_flattened_errors() {
    for query in ["0", "101", "1.5", "NaN", "", "-1"] {
        let (status, error) = request("GET", &format!("{PATH}?limit={query}"), "", true).await;
        assert_eq!(status, 400);
        assert!(error["error"]["fieldErrors"]["limit"].is_array());
    }
    for body in [
        r#"{}"#,
        r#"{"totalUsd":"12"}"#,
        r#"{"totalUsd":null}"#,
        r#"{"totalUsd":-1}"#,
        r#"{"totalUsd":1e1000}"#,
        r#"{"totalUsd":10000000000000000}"#,
    ] {
        let (status, error) = request("POST", PATH, body, true).await;
        assert_eq!(status, 400, "{body}");
        assert!(error["error"]["fieldErrors"]["totalUsd"].is_array());
    }
    assert_eq!(request("POST", PATH, "[]", true).await.0, 400);
    assert_eq!(request("POST", PATH, "{", true).await.0, 400);
}
