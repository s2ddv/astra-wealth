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
