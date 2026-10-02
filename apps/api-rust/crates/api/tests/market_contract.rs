use application::{asset_icon::AssetIconService, market::CryptoMarketService};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use domain::{
    asset_icon::{AssetCache, AssetIconProvider, AssetRef, IconError, IconFuture, IconUrl},
    market::{CryptoMarketProvider, MarketCoin, MarketQuery},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tower::ServiceExt;

struct NoIcons;
impl AssetIconProvider for NoIcons {
    fn resolve_icon<'a>(&'a self, _: &'a AssetRef) -> IconFuture<'a, Option<IconUrl>> {
        Box::pin(async { Ok(None) })
    }
}
impl AssetCache for NoIcons {
    fn get<'a>(&'a self, _: &'a str) -> IconFuture<'a, Option<String>> {
        Box::pin(async { Ok(None) })
    }
    fn set<'a>(&'a self, _: &'a str, _: &'a str, _: u64) -> IconFuture<'a, ()> {
        Box::pin(async { Ok(()) })
    }
}
struct Markets {
    calls: Arc<AtomicUsize>,
    fail: bool,
}
fn bitcoin() -> MarketCoin {
    MarketCoin {
        id: "bitcoin".into(),
        symbol: "btc".into(),
        name: "Bitcoin".into(),
        price_usd: Some(60000.0),
        change_24h: Some(-1.25),
        market_cap: Some(1e12),
        volume_24h: Some(20e9),
        rank: Some(1),
        updated_at: Some("2026-09-27T12:00:00Z".into()),
        ..MarketCoin::default()
    }
}
impl CryptoMarketProvider for Markets {
    fn market_page(&self, query: MarketQuery) -> IconFuture<'_, Vec<MarketCoin>> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                return Err(IconError::Unavailable);
            }
            assert!((1..=100).contains(&query.per_page));
            Ok(vec![bitcoin()])
        })
    }
    fn trending(&self) -> IconFuture<'_, Vec<MarketCoin>> {
        Box::pin(async { Ok(vec![bitcoin()]) })
    }
    fn coin<'a>(&'a self, id: &'a str) -> IconFuture<'a, MarketCoin> {
        Box::pin(async move {
            match id {
                "missing" => Err(IconError::NotFound),
                "limited" => Err(IconError::RateLimited),
                _ => Ok(bitcoin()),
            }
        })
    }
}
fn app(fail: bool) -> (axum::Router, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let icons = AssetIconService::crypto(Arc::new(NoIcons), Arc::new(NoIcons), Arc::new(NoIcons));
    (
        astra_api::market::router(CryptoMarketService::new(
            Arc::new(Markets {
                calls: calls.clone(),
                fail,
            }),
            icons,
        )),
        calls,
    )
}
async fn get(app: axum::Router, path: &str) -> axum::response::Response {
    app.oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
}
#[tokio::test]
async fn list_and_legacy_spot_preserve_values_and_nullable_icons() {
    let (app, _) = app(false);
    let response = get(app.clone(), "/v1/market/coins?page=2&perPage=1&ids=bitcoin").await;
    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(json["page"], 2);
    assert_eq!(json["hasMore"], true);
    assert_eq!(json["data"][0]["currentPrice"], 60000.0);
    assert_eq!(json["data"][0]["priceChangePercentage24h"], -1.25);
    assert!(json["data"][0]["image"].is_null());
    let response = get(app, "/v1/market/spot?limit=1").await;
    let json: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert!(json.is_array());
    assert_eq!(json[0]["id"], "bitcoin");
}
#[tokio::test]
async fn invalid_queries_never_reach_provider_and_no_stock_routes_exist() {
    let (app, calls) = app(false);
    for path in [
        "/v1/market/coins?page=0",
        "/v1/market/coins?perPage=101",
        "/v1/market/coins?page=bad",
        "/v1/market/coins?ids=bitcoin,",
        "/v1/market/coins?ids=..",
        "/v1/market/spot?limit=0",
        "/v1/market/coins?unknown=1",
    ] {
        assert_eq!(
            get(app.clone(), path).await.status(),
            StatusCode::BAD_REQUEST,
            "{path}"
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    for path in ["/stocks", "/v1/stocks", "/v1/market/stocks"] {
        assert_eq!(get(app.clone(), path).await.status(), StatusCode::NOT_FOUND);
    }
}
#[tokio::test]
async fn details_trending_and_failures_have_explicit_statuses() {
    let (router, _) = app(false);
    assert_eq!(
        get(router.clone(), "/v1/market/trending").await.status(),
        StatusCode::OK
    );
    assert_eq!(
        get(router.clone(), "/v1/market/coins/bitcoin")
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        get(router.clone(), "/v1/market/coins/missing")
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    let limited = get(router, "/v1/market/coins/limited").await;
    assert_eq!(limited.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(limited.headers()["retry-after"], "60");
    assert_eq!(
        get(app(true).0, "/v1/market/coins").await.status(),
        StatusCode::BAD_GATEWAY
    );
}
