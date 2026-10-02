use application::UserService;
use astra_api::{
    auth::{AuthState, AuthenticatedUser},
    user::UserDto,
};
use axum::{
    Json, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    routing::get,
};
use domain::{AuthIdentity, NewUser, RepositoryFuture, User, UserRepository, UserUpdate};
use infrastructure::auth::SupabaseAuth;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde_json::{Value, json};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tower::ServiceExt;

const SECRET: &str = "test-only-secret-not-a-real-supabase-secret";
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
async fn inspect(AuthenticatedUser(user): AuthenticatedUser) -> Json<UserDto> {
    Json(user.into())
}
fn protected(
    verifier: Option<Arc<SupabaseAuth>>,
    production: bool,
    users: Arc<MemoryUsers>,
) -> Router {
    // Test-only route: deliberately absent from the production router.
    Router::new()
        .route("/__test_auth", get(inspect))
        .with_state(AuthState {
            users: UserService::new(users),
            verifier,
            production,
            dev_user_id: Some("demo_user_zora".into()),
        })
}
async fn request(app: Router, token: Option<&str>) -> (StatusCode, Value) {
    let mut request = Request::builder().uri("/__test_auth");
    if let Some(token) = token {
        request = request.header("authorization", token);
    }
    let response = app
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}
async fn mock_supabase() -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
    let hits = Arc::new(AtomicUsize::new(0));
    let seen = hits.clone();
    let app = Router::new()
        .route(
            "/auth/v1/user",
            get(move || async move {
                seen.fetch_add(1, Ordering::SeqCst);
                Json(json!({"id":"supabase-id","email":"current@example.com"}))
            }),
        )
        .route(
            "/auth/v1/.well-known/jwks.json",
            get(|| async {
                Json(serde_json::from_str::<Value>(include_str!("fixtures/jwks.json")).unwrap())
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (url, hits, task)
}
fn claims(url: &str) -> Value {
    json!({"sub":"supabase-id","email":"stale@example.com","aud":"authenticated",
        "iss":format!("{url}/auth/v1"),"exp":chrono::Utc::now().timestamp()+3600})
}
fn hs_token(claims: &Value, secret: &str, algorithm: Algorithm) -> String {
    encode(
        &Header::new(algorithm),
        claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap()
}

#[tokio::test]
async fn dev_identity_is_disabled_in_production_and_invalid_tokens_never_fall_back() {
    let users = Arc::new(MemoryUsers::default());
    let (status, body) = request(protected(None, false, users.clone()), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({"id":"demo_user_zora","email":"current@example.com","name":null,
        "authId":"supabase-id","createdAt":"2023-11-14T22:13:20.123Z","updatedAt":"2023-11-14T22:13:20.123Z"})
    );
    let (status, body) = request(protected(None, true, users.clone()), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        body,
        json!({"error":"Missing or invalid authorization header"})
    );
    let (status, _) = request(
        protected(
            Some(Arc::new(
                SupabaseAuth::new(
                    "http://127.0.0.1:1",
                    Some(SECRET.into()),
                    None,
                    Some("test".into()),
                )
                .unwrap(),
            )),
            false,
            users,
        ),
        Some("Bearer broken"),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn hs256_and_es256_preserve_current_supabase_identity_and_validate_claims() {
    let (url, hits, task) = mock_supabase().await;
    let verifier = Arc::new(
        SupabaseAuth::new(
            &url,
            Some(SECRET.into()),
            None,
            Some("test-service-key".into()),
        )
        .unwrap(),
    );
    let users = Arc::new(MemoryUsers::default());
    let app = protected(Some(verifier), true, users.clone());
    let valid = hs_token(&claims(&url), SECRET, Algorithm::HS256);
    assert_eq!(
        request(app.clone(), Some(&format!("Bearer {valid}")))
            .await
            .0,
        StatusCode::OK
    );
    // Legacy also accepts a raw JWT without Bearer prefix.
    assert_eq!(request(app.clone(), Some(&valid)).await.0, StatusCode::OK);
    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some("test-ec".into());
    let key = EncodingKey::from_ec_pem(include_bytes!("fixtures/test-ec-private.pem")).unwrap();
    let es = encode(&header, &claims(&url), &key).unwrap();
    assert_eq!(
        request(app.clone(), Some(&format!("Bearer {es}"))).await.0,
        StatusCode::OK
    );
    assert_eq!(users.identities.lock().unwrap().len(), 3);
    assert_eq!(
        users.identities.lock().unwrap()[0].email.as_deref(),
        Some("current@example.com")
    );
    let mut invalid_claims = Vec::new();
    for (field, value) in [
        ("exp", json!(1)),
        ("aud", json!("wrong")),
        ("iss", json!("https://wrong/auth/v1")),
        ("sub", json!("")),
        ("nbf", json!(chrono::Utc::now().timestamp() + 3600)),
    ] {
        let mut claim = claims(&url);
        claim[field] = value;
        invalid_claims.push(claim);
    }
    for field in ["exp", "aud", "iss", "sub"] {
        let mut claim = claims(&url);
        claim.as_object_mut().unwrap().remove(field);
        invalid_claims.push(claim);
    }
    let mut invalid: Vec<String> = invalid_claims
        .iter()
        .map(|claims| hs_token(claims, SECRET, Algorithm::HS256))
        .collect();
    invalid.push(hs_token(&claims(&url), "wrong-secret", Algorithm::HS256));
    invalid.push(hs_token(&claims(&url), SECRET, Algorithm::HS384));
    header.kid = Some("unknown-key".into());
    invalid.push(encode(&header, &claims(&url), &key).unwrap());
    invalid.push("malformed.jwt".into());
    for token in invalid {
        let (status, body) = request(app.clone(), Some(&format!("Bearer {token}"))).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body, json!({"error":"Invalid or expired token"}));
    }
    assert_eq!(hits.load(Ordering::SeqCst), 3);
    task.abort();
}

#[tokio::test]
async fn legacy_x_user_id_and_missing_auth_errors_match_fastify() {
    let state = AuthState {
        users: UserService::new(Arc::new(MemoryUsers::default())),
        verifier: None,
        production: false,
        dev_user_id: None,
    };
    let app = Router::new()
        .route("/__test_auth", get(inspect))
        .with_state(state);
    let (status, body) = request(app.clone(), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        body,
        json!({"error":"Authentication required",
        "hint":"Use Bearer token in Authorization header or set DEV_USER_ID in .env"})
    );
    for (id, expected) in [
        ("demo_user_zora", StatusCode::OK),
        ("unknown", StatusCode::UNAUTHORIZED),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/__test_auth")
                    .header("x-user-id", id)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        if expected == StatusCode::UNAUTHORIZED {
            let value: Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap())
                    .unwrap();
            assert_eq!(value, json!({"error":"Invalid X-User-Id header"}));
        }
    }
}

#[tokio::test]
async fn deleted_supabase_users_are_rejected_before_upsert() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let upstream = Router::new().route("/auth/v1/user", get(|| async { StatusCode::UNAUTHORIZED }));
    let task = tokio::spawn(async move {
        axum::serve(listener, upstream).await.unwrap();
    });
    let users = Arc::new(MemoryUsers::default());
    let auth = Arc::new(
        SupabaseAuth::new(
            &url,
            Some(SECRET.into()),
            None,
            Some("test-service-key".into()),
        )
        .unwrap(),
    );
    let token = hs_token(&claims(&url), SECRET, Algorithm::HS256);
    let (status, body) = request(protected(Some(auth), true, users.clone()), Some(&token)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, json!({"error":"Invalid or expired token"}));
    assert!(users.identities.lock().unwrap().is_empty());
    task.abort();
}
