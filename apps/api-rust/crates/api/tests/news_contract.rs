use application::{UserService, news::NewsService};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use domain::{AuthIdentity, NewUser, RepositoryFuture, User, UserRepository, UserUpdate, news::*};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use zora_api::{auth::AuthState, news::router};
fn demo_user() -> User {
    let time = chrono::DateTime::from_timestamp_millis(1_700_000_000_123)
        .unwrap()
        .naive_utc();
    User {
        id: "demo_user_zora".into(),
        email: "current@example.com".into(),
        name: None,
        auth_id: "supabase-id".into(),
        created_at: time,
        updated_at: time,
    }
}
#[derive(Default)]
struct MemoryUsers {
    identities: Mutex<Vec<AuthIdentity>>,
}
impl UserRepository for MemoryUsers {
    fn find_by_id<'a>(&'a self, id: &'a str) -> RepositoryFuture<'a, Option<User>> {
        Box::pin(async move { Ok((id == "demo_user_zora").then(demo_user)) })
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
    fn upsert_identity(&self, identity: AuthIdentity) -> RepositoryFuture<'_, User> {
        self.identities.lock().unwrap().push(identity);
        Box::pin(async { Ok(demo_user()) })
    }
}

struct EmptyCache;
impl NewsCache for EmptyCache {
    fn get<'a>(&'a self, _: &'a str, _: bool) -> NewsFuture<'a, Option<Vec<NewsArticle>>> {
        Box::pin(async { Ok(None) })
    }
    fn put<'a>(&'a self, _: &'a str, _: &'a [NewsArticle]) -> NewsFuture<'a, ()> {
        Box::pin(async { Ok(()) })
    }
}
struct FixtureProvider;
impl NewsProvider for FixtureProvider {
    fn fetch(&self, _: NewsFilter) -> NewsFuture<'_, Vec<NewsArticle>> {
        Box::pin(async {
            Ok(vec![NewsArticle {
                id: "test".into(),
                title: "Test".into(),
                summary: Some("Summary".into()),
                url: "https://example.com/news".into(),
                image_url: None,
                source_name: "Fixture".into(),
                published_at: "2026-09-28T12:00:00Z".into(),
                category: NewsCategory::Crypto,
                language: NewsLanguage::En,
                related_symbols: vec![],
            }])
        })
    }
}
fn app(authenticated: bool) -> axum::Router {
    router(
        AuthState {
            users: UserService::new(Arc::new(MemoryUsers::default())),
            verifier: None,
            production: false,
            dev_user_id: authenticated.then(|| "demo_user_zora".into()),
        },
        NewsService::new(vec![Arc::new(FixtureProvider)], Arc::new(EmptyCache)),
    )
}
#[tokio::test]
async fn authenticated_news_contract_and_invalid_queries() {
    let response = app(false)
        .oneshot(
            Request::builder()
                .uri("/v1/news")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    for query in [
        "limit=0",
        "limit=51",
        "limit=-1",
        "category=stocks",
        "lang=es",
        "cursor=invalid",
        "unknown=x",
    ] {
        let response = app(true)
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/news?{query}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{query}");
    }
    let response = app(true)
        .oneshot(
            Request::builder()
                .uri("/v1/news?category=crypto&lang=en&limit=1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(body["data"][0]["sourceName"], "Fixture");
    assert_eq!(body["data"][0]["relatedSymbols"], serde_json::json!([]));
    assert!(body["nextCursor"].is_null());
    assert_eq!(body["stale"], false);
    let response = app(true)
        .oneshot(
            Request::builder()
                .uri("/v1/news?lang=pt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(body["data"], serde_json::json!([]));
}
