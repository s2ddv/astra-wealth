use super::*;
use axum::{Json, Router, routing::post};
use domain::asset_icon::{AssetCache, IconFuture};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
#[derive(Default)]
struct Cache(Mutex<HashMap<String, (String, u64)>>);
impl AssetCache for Cache {
    fn get<'a>(&'a self, key: &'a str) -> IconFuture<'a, Option<String>> {
        Box::pin(async move { Ok(self.0.lock().unwrap().get(key).map(|v| v.0.clone())) })
    }
    fn set<'a>(&'a self, key: &'a str, value: &'a str, ttl: u64) -> IconFuture<'a, ()> {
        Box::pin(async move {
            self.0
                .lock()
                .unwrap()
                .insert(key.into(), (value.into(), ttl));
            Ok(())
        })
    }
}
async fn server(app: Router) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    (
        url,
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() }),
    )
}
#[tokio::test]
async fn alchemy_retries_429_and_caches_metadata() {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let app=Router::new().route("/",post(move |Json(body):Json<Value>| {let count=count.clone();async move {
        if count.fetch_add(1,Ordering::SeqCst)==0 {return (axum::http::StatusCode::TOO_MANY_REQUESTS,Json(json!({"secret":"must not escape"})));}
        assert_eq!(body["jsonrpc"],"2.0");
        let result=match body["method"].as_str().unwrap() {
            "eth_getBalance" => {assert_eq!(body["params"][1],"latest");json!("0xde0b6b3a7640000")},
            "alchemy_getTokenMetadata" => json!({"name":"USD Coin","symbol":"USDC","decimals":6}),
            "alchemy_getTokenBalances" => { assert_eq!(body["params"][1],"erc20");json!({"tokenBalances":[{"contractAddress":"0x0000000000000000000000000000000000000001","tokenBalance":"0x0"}],"pageKey":"next"}) },
            _=>panic!("unexpected RPC")
        };
        (axum::http::StatusCode::OK,Json(json!({"jsonrpc":"2.0","id":1,"result":result})))
    }}));
    let (url, task) = server(app).await;
    let cache = Arc::new(Cache::default());
    let mut provider = AlchemyProvider::new(asset_http_client().unwrap(), cache.clone(), None);
    provider.test_url = Some(url);
    assert_eq!(
        provider
            .native_balance(domain::wallet::Chain::Ethereum, "wallet")
            .await
            .unwrap(),
        "0xde0b6b3a7640000"
    );
    let chain = domain::wallet::Chain::Ethereum;
    for _ in 0..2 {
        assert_eq!(
            provider
                .token_metadata(chain, "token")
                .await
                .unwrap()
                .unwrap()
                .decimals,
            6
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(
        cache.0.lock().unwrap().values().next().unwrap().1,
        METADATA_TTL
    );
    assert!(
        !provider
            .token_balances(chain, "wallet")
            .await
            .unwrap()
            .complete
    );
    task.abort();
}
#[tokio::test]
async fn alchemy_bounded_retries_and_redacted_errors() {
    for (status, rate) in [(429, true), (503, false), (400, false)] {
        let calls = Arc::new(AtomicUsize::new(0));
        let c = calls.clone();
        let app = Router::new().route(
            "/",
            post(move || {
                let c = c.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    (
                        axum::http::StatusCode::from_u16(status).unwrap(),
                        "secret API key",
                    )
                }
            }),
        );
        let (url, task) = server(app).await;
        let mut p = AlchemyProvider::new(
            asset_http_client().unwrap(),
            Arc::new(Cache::default()),
            None,
        );
        p.test_url = Some(url);
        let err = p
            .native_balance(domain::wallet::Chain::Base, "address")
            .await
            .unwrap_err();
        assert_eq!(matches!(err, TrackingError::RateLimited), rate);
        assert!(!err.to_string().contains("secret"));
        assert_eq!(
            calls.load(Ordering::SeqCst),
            if status == 400 { 1 } else { 3 }
        );
        task.abort();
    }
}
#[tokio::test]
async fn prices_preserve_precision_missing_price_and_ten_second_cache() {
    use axum::routing::get;
    let count = Arc::new(AtomicUsize::new(0));
    let c = count.clone();
    let app = Router::new()
        .route(
            "/simple/price",
            get(move || {
                let c = c.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    axum::response::Response::builder()
                        .header("content-type", "application/json")
                        .body(axum::body::Body::from(
                            r#"{"ethereum":{"usd":1234.123456789123456789}}"#,
                        ))
                        .unwrap()
                }
            }),
        )
        .route(
            "/simple/token_price/base",
            get(|| async { Json(json!({})) }),
        );
    let (url, task) = server(app).await;
    let cache = Arc::new(Cache::default());
    let mut p = CoinGeckoPriceProvider::new(asset_http_client().unwrap(), cache.clone(), None);
    p.base = url;
    for _ in 0..2 {
        assert_eq!(
            p.native_price(domain::wallet::Chain::Base)
                .await
                .unwrap()
                .as_deref(),
            Some("1234.123456789123456789")
        );
    }
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert!(
        p.token_prices(domain::wallet::Chain::Base, &["0x1".into()])
            .await
            .unwrap()
            .is_empty()
    );
    assert!(cache.0.lock().unwrap().values().all(|v| v.1 == PRICE_TTL));
    task.abort();
}
#[test]
fn network_mapping_and_secret_configuration() {
    use domain::wallet::Chain;
    for (chain, network) in [
        (Chain::Ethereum, "eth-mainnet"),
        (Chain::Polygon, "polygon-mainnet"),
        (Chain::Arbitrum, "arb-mainnet"),
        (Chain::Base, "base-mainnet"),
    ] {
        assert_eq!(chain_info(chain).unwrap().network, network);
    }
    assert!(matches!(
        chain_info(Chain::Solana),
        Err(TrackingError::ChainNotSupported)
    ));
    assert!(matches!(
        chain_info(Chain::Bitcoin),
        Err(TrackingError::ChainNotSupported)
    ));
    assert_eq!(
        chain_info(Chain::Polygon).unwrap().coin_id,
        "polygon-ecosystem-token"
    );
}

#[tokio::test]
async fn portfolio_brl_prices_are_exact_cached_and_separate_from_usd() {
    use application::portfolio::BrlPriceProvider;
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let app = Router::new().route(
        "/simple/price",
        axum::routing::get(
            move |axum::extract::RawQuery(query): axum::extract::RawQuery| {
                let count = count.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    let query: HashMap<_, _> =
                        url::form_urlencoded::parse(query.unwrap().as_bytes())
                            .into_owned()
                            .collect();
                    assert_eq!(query["vs_currencies"], "brl");
                    assert_eq!(query["precision"], "full");
                    r#"{"ethereum":{"brl":12345.123456789012345678}}"#
                }
            },
        ),
    );
    let (url, task) = server(app).await;
    let cache = Arc::new(Cache::default());
    cache.0.lock().unwrap().insert(
        "tracking:price:native:ethereum".into(),
        ("\"999\"".into(), 10),
    );
    let mut provider = CoinGeckoPriceProvider::new(asset_http_client().unwrap(), cache, None);
    provider.base = url;
    for _ in 0..2 {
        let quote = provider
            .price(domain::wallet::Chain::Ethereum, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(quote.price, "12345.123456789012345678");
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    task.abort();
}
