use super::*;
use domain::asset_icon::IconFuture;
use std::{
    collections::HashMap,
    sync::{Mutex, atomic::Ordering},
};

#[derive(Default)]
struct Cache {
    values: Mutex<HashMap<String, String>>,
    writes: Mutex<Vec<(String, u64)>>,
}
impl AssetCache for Cache {
    fn get<'a>(&'a self, key: &'a str) -> IconFuture<'a, Option<String>> {
        Box::pin(async { Ok(self.values.lock().unwrap().get(key).cloned()) })
    }
    fn set<'a>(&'a self, key: &'a str, value: &'a str, ttl: u64) -> IconFuture<'a, ()> {
        Box::pin(async move {
            self.values.lock().unwrap().insert(key.into(), value.into());
            self.writes.lock().unwrap().push((key.into(), ttl));
            Ok(())
        })
    }
}
#[tokio::test]
async fn one_market_request_feeds_prices_and_icon_cache_then_reuses_price_cache() {
    let body = br#"[{"id":"bitcoin","symbol":"btc","name":"Bitcoin","image":"https://coin-images.coingecko.com/btc.png","current_price":67000}]"#;
    let (base, calls, task) = crate::asset_icon::tests::server(
        axum::http::StatusCode::OK,
        body.to_vec(),
        "application/json",
    )
    .await;
    let cache = Arc::new(Cache::default());
    let primary = Arc::new(CoinGeckoIconProvider::default());
    let client = asset_http_client().unwrap();
    let mut provider =
        CoinGeckoMarketProvider::new(client.clone(), cache.clone(), primary.clone(), None);
    provider.base = base;
    let icons = AssetIconService::crypto(
        primary,
        Arc::new(TrustWalletIconProvider::new(client)),
        cache.clone(),
    );
    let market = CryptoMarketService::new(Arc::new(provider), icons);
    for _ in 0..2 {
        let result = market.markets().await.unwrap();
        assert_eq!(result[0].0.price_usd, Some(67000.0));
        assert_eq!(
            result[0].1.url.as_ref().unwrap().as_str(),
            "https://coin-images.coingecko.com/btc.png"
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let writes = cache.writes.lock().unwrap();
    assert_eq!(
        writes.iter().filter(|(_, ttl)| *ttl == 60).count(),
        1,
        "price cache hits must not extend its TTL"
    );
    assert_eq!(
        writes
            .iter()
            .filter(|(key, ttl)| key == "icon:crypto:id:bitcoin" && *ttl == 86400)
            .count(),
        2
    );
    task.abort();
}
#[tokio::test]
async fn coin_detail_extracts_large_then_small_and_does_not_make_an_icon_call() {
    for (images, expected) in [
        (
            r#"{"large":"https://coin-images.coingecko.com/large.png","small":"https://coin-images.coingecko.com/small.png"}"#,
            "large",
        ),
        (
            r#"{"large":null,"small":"https://coin-images.coingecko.com/small.png"}"#,
            "small",
        ),
    ] {
        let body = format!(
            r#"{{"id":"bitcoin","symbol":"btc","name":"Bitcoin","image":{images},"market_data":{{"current_price":{{"usd":67000}}}}}}"#
        );
        let (base, calls, task) = crate::asset_icon::tests::server(
            axum::http::StatusCode::OK,
            body.into_bytes(),
            "application/json",
        )
        .await;
        let cache = Arc::new(Cache::default());
        let primary = Arc::new(CoinGeckoIconProvider::default());
        let mut provider =
            CoinGeckoMarketProvider::new(asset_http_client().unwrap(), cache, primary, None);
        provider.base = base;
        let coin = provider.coin("bitcoin").await.unwrap();
        assert_eq!(coin.price_usd, Some(67000.0));
        assert!(coin.image.unwrap().as_str().contains(expected));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(provider.coin("../secret").await.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        task.abort();
    }
}
#[tokio::test]
async fn malformed_payload_and_upstream_failure_are_not_cached() {
    for (status, body) in [
        (axum::http::StatusCode::OK, b"not json".to_vec()),
        (axum::http::StatusCode::TOO_MANY_REQUESTS, b"[]".to_vec()),
    ] {
        let (base, _, task) =
            crate::asset_icon::tests::server(status, body, "application/json").await;
        let cache = Arc::new(Cache::default());
        let mut provider = CoinGeckoMarketProvider::new(
            asset_http_client().unwrap(),
            cache.clone(),
            Arc::new(CoinGeckoIconProvider::default()),
            None,
        );
        provider.base = base;
        assert!(provider.markets().await.is_err());
        assert!(cache.writes.lock().unwrap().is_empty());
        task.abort();
    }
}

#[tokio::test]
async fn pagination_and_ids_are_forwarded_and_cache_fills_are_coalesced() {
    use axum::{Router, extract::Query, routing::get};
    use std::sync::atomic::AtomicUsize;
    let calls = Arc::new(AtomicUsize::new(0));
    let captured = Arc::new(Mutex::new(Vec::new()));
    let count = calls.clone();
    let requests = captured.clone();
    let app = Router::new().route(
        "/coins/markets",
        get(move |Query(params): Query<HashMap<String, String>>| {
            let count = count.clone();
            let requests = requests.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                requests.lock().unwrap().push(params);
                axum::Json(serde_json::json!([]))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let mut provider = CoinGeckoMarketProvider::new(
        asset_http_client().unwrap(),
        Arc::new(Cache::default()),
        Arc::new(CoinGeckoIconProvider::default()),
        None,
    );
    provider.base = base;
    let query = MarketQuery {
        page: 2,
        per_page: 50,
        ids: vec!["ethereum".into(), "bitcoin".into(), "bitcoin".into()],
    };
    let (first, second) = tokio::join!(
        provider.market_page(query.clone()),
        provider.market_page(query)
    );
    assert!(first.is_ok() && second.is_ok());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let requests = captured.lock().unwrap();
    assert_eq!(requests[0]["page"], "2");
    assert_eq!(requests[0]["per_page"], "50");
    assert_eq!(requests[0]["ids"], "bitcoin,ethereum");
    assert_eq!(requests[0]["vs_currency"], "usd");
    task.abort();
}
#[tokio::test]
async fn trending_uses_its_own_payload_and_five_minute_cache() {
    let body = br#"{"coins":[{"item":{"id":"bitcoin","symbol":"BTC","name":"Bitcoin","large":"https://coin-images.coingecko.com/btc.png","market_cap_rank":1}}]}"#;
    let (base, calls, task) = crate::asset_icon::tests::server(
        axum::http::StatusCode::OK,
        body.to_vec(),
        "application/json",
    )
    .await;
    let cache = Arc::new(Cache::default());
    let mut provider = CoinGeckoMarketProvider::new(
        asset_http_client().unwrap(),
        cache.clone(),
        Arc::new(CoinGeckoIconProvider::default()),
        None,
    );
    provider.base = base;
    for _ in 0..2 {
        let coins = provider.trending().await.unwrap();
        assert_eq!(coins[0].id, "bitcoin");
        assert!(coins[0].price_usd.is_none());
        assert!(coins[0].image.is_some());
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(cache.writes.lock().unwrap()[0].1, 300);
    task.abort();
}
#[tokio::test]
async fn upstream_rate_limit_activates_cooldown_and_invalid_cache_recovers() {
    let (base, calls, task) = crate::asset_icon::tests::server(
        axum::http::StatusCode::TOO_MANY_REQUESTS,
        vec![],
        "application/json",
    )
    .await;
    let mut provider = CoinGeckoMarketProvider::new(
        asset_http_client().unwrap(),
        Arc::new(Cache::default()),
        Arc::new(CoinGeckoIconProvider::default()),
        None,
    );
    provider.base = base;
    for _ in 0..2 {
        assert!(matches!(
            provider.markets().await,
            Err(IconError::RateLimited)
        ));
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    task.abort();
    let cache = Arc::new(Cache::default());
    cache
        .values
        .lock()
        .unwrap()
        .insert("market:coingecko:usd:v2:1:100:".into(), "bad json".into());
    let (base, calls, task) = crate::asset_icon::tests::server(
        axum::http::StatusCode::OK,
        b"[]".to_vec(),
        "application/json",
    )
    .await;
    let mut provider = CoinGeckoMarketProvider::new(
        asset_http_client().unwrap(),
        cache,
        Arc::new(CoinGeckoIconProvider::default()),
        None,
    );
    provider.base = base;
    assert!(provider.markets().await.unwrap().is_empty());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    task.abort();
}
