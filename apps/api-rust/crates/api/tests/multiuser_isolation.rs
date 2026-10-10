//! Real HTTP handlers + JWT verification + SQL repositories + Redis.
//! Only the identity provider and paid market/on-chain providers are simulated.
use application::{UserService, accounts::AccountService, tracking::*, wallet::WalletService, watchlist::WatchlistService};
use astra_api::{accounts::{self, AccountState}, auth::AuthState, tracking, wallet::{self, WalletState}, watchlist::{self, WatchlistState}};
use axum::{Json, Router, body::{Body, to_bytes}, http::{HeaderMap, Request, StatusCode}, routing::get};
use domain::wallet::Chain;
use infrastructure::{accounts::SqlxAccountRepository, auth::SupabaseAuth, tracking::{RedisTrackingCache, SqlxTrackingRepository}, user::SqlxUserRepository, wallet::SqlxWalletRepository, watchlist::SqlxWatchlistRepository};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::{Arc, atomic::{AtomicUsize, Ordering}}};
use tower::ServiceExt;

const SUBJECTS: [&str; 2] = ["isolation-auth-a", "isolation-auth-b"];
const OWNERS: [&str; 2] = ["isolation-local-a", "isolation-local-b"];
const SECRET: &str = "synthetic-isolation-signing-secret";

struct Providers(AtomicUsize);
impl OnChainProvider for Providers {
    fn native_balance<'a>(&'a self, _: Chain, _: &'a str) -> TrackingFuture<'a, String> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Ok("0xde0b6b3a7640000".into()) })
    }
    fn token_balances<'a>(&'a self, _: Chain, _: &'a str) -> TrackingFuture<'a, TokenBalances> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Ok(TokenBalances { tokens: vec![], complete: true }) })
    }
    fn token_metadata<'a>(&'a self, _: Chain, _: &'a str) -> TrackingFuture<'a, Option<TokenMetadata>> {
        panic!("No token metadata should be requested for the empty token fixture")
    }
}
impl PriceProvider for Providers {
    fn native_price(&self, _: Chain) -> TrackingFuture<'_, Option<String>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Ok(Some("100".into())) })
    }
    fn token_prices<'a>(&'a self, _: Chain, _: &'a [String]) -> TrackingFuture<'a, HashMap<String, String>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Ok(HashMap::new()) })
    }
}

struct Cache { inner: RedisTrackingCache, calls: AtomicUsize }
impl TrackingCache for Cache {
    fn get<'a>(&'a self, key: &'a str) -> TrackingFuture<'a, Option<TrackedWallet>> {
        self.calls.fetch_add(1, Ordering::SeqCst); self.inner.get(key)
    }
    fn put<'a>(&'a self, key: &'a str, wallet: &'a TrackedWallet) -> TrackingFuture<'a, ()> {
        self.calls.fetch_add(1, Ordering::SeqCst); self.inner.put(key, wallet)
    }
    fn invalidate<'a>(&'a self, key: &'a str) -> TrackingFuture<'a, ()> {
        self.calls.fetch_add(1, Ordering::SeqCst); self.inner.invalidate(key)
    }
    fn acquire<'a>(&'a self, key: &'a str) -> TrackingFuture<'a, Option<String>> {
        self.calls.fetch_add(1, Ordering::SeqCst); self.inner.acquire(key)
    }
    fn release<'a>(&'a self, key: &'a str, token: &'a str) -> TrackingFuture<'a, ()> {
        self.calls.fetch_add(1, Ordering::SeqCst); self.inner.release(key, token)
    }
    fn allow_sync<'a>(&'a self, key: &'a str) -> TrackingFuture<'a, bool> {
        self.calls.fetch_add(1, Ordering::SeqCst); self.inner.allow_sync(key)
    }
}

async fn identity_provider() -> (String, Vec<String>, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let tokens: Vec<_> = SUBJECTS.iter().map(|subject| encode(&Header::new(Algorithm::HS256),
        &json!({"sub":subject,"aud":"authenticated","iss":format!("{url}/auth/v1"),"exp":chrono::Utc::now().timestamp()+3600}),
        &EncodingKey::from_secret(SECRET.as_bytes())).unwrap()).collect();
    let accepted = tokens.clone();
    let mock = Router::new().route("/auth/v1/user", get(move |headers: HeaderMap| {
        let accepted = accepted.clone();
        async move {
            let token = headers.get("authorization").and_then(|v| v.to_str().ok()).unwrap_or("");
            match accepted.iter().position(|candidate| token == format!("Bearer {candidate}")) {
                Some(i) => (StatusCode::OK, Json(json!({"id":SUBJECTS[i],"email":format!("{}@test.invalid", SUBJECTS[i])}))),
                None => (StatusCode::UNAUTHORIZED, Json(json!({"error":"invalid token"}))),
            }
        }
    }));
    let task = tokio::spawn(async move { axum::serve(listener, mock).await.unwrap(); });
    (url, tokens, task)
}

async fn request(app: &Router, token: &str, method: &str, path: &str, body: Value, expected: u16) -> Value {
    let response = app.clone().oneshot(Request::builder().method(method).uri(path)
        .header("authorization", format!("Bearer {token}"))
        .header("x-user-id", "forged-owner")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string())).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1_048_576).await.unwrap();
    assert_eq!(status.as_u16(), expected, "{method} {path}: {}", String::from_utf8_lossy(&bytes));
    if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap() }
}
fn id(value: &Value) -> &str { value["id"].as_str().unwrap() }
fn holding() -> Value { json!({"assetClass":"CASH","name":"Cash","quantity":"1.25","currentValue":"125.50"}) }
fn contribution() -> Value { json!({"kind":"DEPOSIT","amount":"40.25","currency":"BRL","occurredAt":"2020-01-01T12:00:00Z"}) }

#[tokio::test]
#[ignore = "Run npm run test:security: creates disposable PostgreSQL and Redis"]
async fn two_verified_users_cannot_cross_parent_child_wallet_watchlist_or_cache_boundaries() {
    let url = std::env::var("TEST_DATABASE_URL").expect("Use npm run test:security");
    assert!(url.contains("@127.0.0.1:") && url.ends_with("/astra_isolation_test"), "Disposable database required");
    let pool = infrastructure::postgres_pool(&url).unwrap();
    let database: String = sqlx::query_scalar("SELECT current_database()").fetch_one(&pool).await.unwrap();
    assert_eq!(database, "astra_isolation_test");
    for i in 0..2 {
        sqlx::query(r#"INSERT INTO users(id,email,"authId","updatedAt") VALUES($1,$2,$3,now())"#)
            .bind(OWNERS[i]).bind(format!("{}@test.invalid", SUBJECTS[i])).bind(SUBJECTS[i]).execute(&pool).await.unwrap();
    }
    let (auth_url, tokens, mock) = identity_provider().await;
    let auth = AuthState {
        users: UserService::new(Arc::new(SqlxUserRepository::new(pool.clone()))),
        verifier: Some(Arc::new(SupabaseAuth::new(&auth_url, Some(SECRET.into()), None, Some("synthetic-service-key".into())).unwrap())),
        allow_dev_auth: false,
        dev_user_id: Some(OWNERS[1].into()),
    };
    let wallets = Arc::new(SqlxWalletRepository::new(pool.clone()));
    let providers = Arc::new(Providers(AtomicUsize::new(0)));
    let redis_url = std::env::var("TEST_REDIS_URL").expect("Disposable Redis required");
    assert!(redis_url.starts_with("redis://127.0.0.1:"));
    let cache = Arc::new(Cache { inner: RedisTrackingCache::new(infrastructure::redis_pool(&redis_url).unwrap()), calls: AtomicUsize::new(0) });
    let app = wallet::router(WalletState { auth: auth.clone(), wallets: Arc::new(WalletService::new(wallets.clone())) })
        .merge(accounts::router(AccountState { auth: auth.clone(), accounts: AccountService::new(Arc::new(SqlxAccountRepository::new(pool.clone()))) }))
        .merge(watchlist::router(WatchlistState { auth: auth.clone(), watchlists: WatchlistService::new(Arc::new(SqlxWatchlistRepository::new(pool.clone()))) }))
        .merge(tracking::router(auth, WalletTrackingService::new(wallets, providers.clone(), providers.clone(), Arc::new(SqlxTrackingRepository::new(pool.clone())), cache.clone())));

    let mut account = vec![];
    let mut position = vec![];
    let mut deposit = vec![];
    // A owns two distinct parents; B owns one. Both mismatches must fail.
    for actor in [0, 1, 0] {
        let a = request(&app, &tokens[actor], "POST", "/v1/accounts", json!({"name":format!("Account {}", account.len()),"kind":"MANUAL","baseCurrency":"BRL"}), 201).await;
        assert_eq!(a["userId"], OWNERS[actor]);
        position.push(request(&app, &tokens[actor], "POST", &format!("/v1/accounts/{}/holdings", id(&a)), holding(), 201).await);
        deposit.push(request(&app, &tokens[actor], "POST", &format!("/v1/accounts/{}/contributions", id(&a)), contribution(), 201).await);
        account.push(a);
    }
    let mut wallet = vec![];
    let mut list = vec![];
    let mut assets = vec![];
    for actor in 0..2 {
        wallet.push(request(&app, &tokens[actor], "POST", "/v1/me/wallets", json!({"address":"0xde709f2102306220921060314715629080e2fb77","chain":"ETHEREUM","nickname":format!("Wallet {actor}")}), 201).await);
        list.push(request(&app, &tokens[actor], "POST", "/v1/me/watchlists", json!({"name":"Markets"}), 201).await);
        request(&app, &tokens[actor], "POST", &format!("/v1/me/watchlists/{}/items", id(&list[actor])), json!({"coinId":if actor==0 {"bitcoin"} else {"ethereum"}}), 201).await;
        assets.push(request(&app, &tokens[actor], "GET", &format!("/v1/me/wallets/{}/assets", id(&wallet[actor])), Value::Null, 200).await);
        assert_eq!(assets[actor]["walletId"], wallet[actor]["id"]);
        assert!(!assets[actor]["assets"].as_array().unwrap().is_empty());
    }
    let mut before = vec![];
    for (index, owner) in [0, 1, 0].into_iter().enumerate() {
        before.push(request(&app, &tokens[owner], "GET", &format!("/v1/accounts/{}", id(&account[index])), Value::Null, 200).await);
    }
    let mut lists_before = vec![];
    let mut wallets_before = vec![];
    for token in &tokens {
        lists_before.push(request(&app, token, "GET", "/v1/me/watchlists", Value::Null, 200).await);
        wallets_before.push(request(&app, token, "GET", "/v1/me/wallets", Value::Null, 200).await);
    }
    let provider_calls = providers.0.load(Ordering::SeqCst);
    let cache_calls = cache.calls.load(Ordering::SeqCst);
    assert!(provider_calls > 0 && cache_calls > 0);

    for actor in 0..2 {
        let other = 1 - actor;
        let foreign = id(&account[other]);
        for (method, path, body) in [
            ("GET", format!("/v1/accounts/{foreign}"), Value::Null),
            ("DELETE", format!("/v1/accounts/{foreign}"), Value::Null),
            ("POST", format!("/v1/accounts/{foreign}/holdings"), holding()),
            ("PUT", format!("/v1/accounts/{foreign}/holdings/{}", id(&position[other])), holding()),
            ("DELETE", format!("/v1/accounts/{foreign}/holdings/{}", id(&position[other])), Value::Null),
            ("POST", format!("/v1/accounts/{foreign}/contributions"), contribution()),
            ("DELETE", format!("/v1/accounts/{foreign}/contributions/{}", id(&deposit[other])), Value::Null),
            ("PATCH", format!("/v1/me/wallets/{}", id(&wallet[other])), json!({"nickname":"Intruder"})),
            ("DELETE", format!("/v1/me/wallets/{}", id(&wallet[other])), Value::Null),
            ("GET", format!("/v1/me/wallets/{}/assets", id(&wallet[other])), Value::Null),
            ("POST", format!("/v1/me/wallets/{}/sync", id(&wallet[other])), json!({})),
            ("DELETE", format!("/v1/me/watchlists/{}", id(&list[other])), Value::Null),
            ("POST", format!("/v1/me/watchlists/{}/items", id(&list[other])), json!({"coinId":"solana"})),
            ("DELETE", format!("/v1/me/watchlists/{}/items/{}", id(&list[other]), if other==0 {"bitcoin"} else {"ethereum"}), Value::Null),
            ("POST", "/v1/accounts".into(), json!({"name":"Foreign wallet","kind":"WALLET","baseCurrency":"USD","walletId":id(&wallet[other])})),
        ] { request(&app, &tokens[actor], method, &path, body, 404).await; }
        // Own parent + a record belonging to another account/user.
        for child in [other, 2] {
            for (method, kind, record, body) in [
                ("PUT", "holdings", id(&position[child]), holding()),
                ("DELETE", "holdings", id(&position[child]), Value::Null),
                ("DELETE", "contributions", id(&deposit[child]), Value::Null),
            ] {
                request(&app, &tokens[actor], method, &format!("/v1/accounts/{}/{kind}/{record}", id(&account[actor])), body, 404).await;
            }
        }
        let own = request(&app, &tokens[actor], "GET", "/v1/accounts", Value::Null, 200).await;
        assert_eq!(own.as_array().unwrap().len(), if actor == 0 { 2 } else { 1 });
        assert!(own.as_array().unwrap().iter().all(|a| a["userId"] == OWNERS[actor]));
    }
    assert_eq!(providers.0.load(Ordering::SeqCst), provider_calls, "Denied requests reached a provider");
    assert_eq!(cache.calls.load(Ordering::SeqCst), cache_calls, "Denied requests reached Redis");
    for (index, owner) in [0, 1, 0].into_iter().enumerate() {
        assert_eq!(request(&app, &tokens[owner], "GET", &format!("/v1/accounts/{}", id(&account[index])), Value::Null, 200).await, before[index]);
    }
    for actor in 0..2 {
        assert_eq!(request(&app, &tokens[actor], "GET", "/v1/me/watchlists", Value::Null, 200).await, lists_before[actor]);
        assert_eq!(request(&app, &tokens[actor], "GET", "/v1/me/wallets", Value::Null, 200).await, wallets_before[actor]);
        assert_eq!(request(&app, &tokens[actor], "GET", &format!("/v1/me/wallets/{}/assets", id(&wallet[actor])), Value::Null, 200).await, assets[actor]);
        request(&app, &tokens[actor], "PUT", &format!("/v1/accounts/{}/holdings/{}", id(&account[actor]), id(&position[actor])), holding(), 200).await;
        request(&app, &tokens[actor], "DELETE", &format!("/v1/accounts/{}/holdings/{}", id(&account[actor]), id(&position[actor])), Value::Null, 204).await;
        request(&app, &tokens[actor], "DELETE", &format!("/v1/accounts/{}/contributions/{}", id(&account[actor]), id(&deposit[actor])), Value::Null, 204).await;
    }
    assert_eq!(providers.0.load(Ordering::SeqCst), provider_calls, "Warm cache was not reused");
    let users: Vec<(String, String)> = sqlx::query_as(r#"SELECT id,"authId" FROM users ORDER BY id"#).fetch_all(&pool).await.unwrap();
    assert_eq!(users, vec![(OWNERS[0].into(), SUBJECTS[0].into()), (OWNERS[1].into(), SUBJECTS[1].into())]);
    // Fixtures intentionally remain for the following Fastify cross-backend test.
    // The gate always removes the entire disposable database afterwards.
    mock.abort();
    pool.close().await;
}
