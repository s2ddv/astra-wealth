use super::*;
use axum::{
    Router,
    body::Body,
    http::{Response, StatusCode as HttpStatus},
    routing::get,
};
use domain::{
    asset_icon::{CryptoRef, StockRef, TokenChain, TokenRef},
    market::MarketCoin,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

pub(crate) fn png() -> Vec<u8> {
    STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+a7s8AAAAASUVORK5CYII=").unwrap()
}
pub(crate) async fn server(
    status: HttpStatus,
    body: Vec<u8>,
    content_type: &'static str,
) -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let app = Router::new().fallback(get(move || {
        let body = body.clone();
        let counter = counter.clone();
        async move {
            counter.fetch_add(1, Ordering::SeqCst);
            Response::builder()
                .status(status)
                .header("content-type", content_type)
                .body(Body::from(body))
                .unwrap()
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (base, calls, task)
}
fn crypto() -> AssetRef {
    AssetRef::Crypto(CryptoRef {
        coingecko_id: None,
        symbol: "usdt".into(),
        token: Some(TokenRef {
            chain: TokenChain::Ethereum,
            contract: "0xdac17f958d2ee523a2206206994597c13d831ec7".into(),
        }),
    })
}
#[tokio::test]
async fn trust_wallet_downloads_verified_png_and_never_exposes_github() {
    let (base, calls, task) = server(HttpStatus::OK, png(), "image/png").await;
    let provider = TrustWalletIconProvider {
        client: asset_http_client().unwrap(),
        base,
    };
    let result = provider.resolve_icon(&crypto()).await.unwrap().unwrap();
    assert_eq!(
        result.as_str(),
        format!("data:image/png;base64,{}", STANDARD.encode(png()))
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    task.abort();
}
#[tokio::test]
async fn http_misses_redirects_and_invalid_images_yield_none() {
    for (status, body, content_type) in [
        (HttpStatus::NOT_FOUND, png(), "image/png"),
        (HttpStatus::TOO_MANY_REQUESTS, png(), "image/png"),
        (HttpStatus::INTERNAL_SERVER_ERROR, png(), "image/png"),
        (HttpStatus::FOUND, png(), "image/png"),
        (HttpStatus::OK, b"<html>error</html>".to_vec(), "text/html"),
        (HttpStatus::OK, vec![0; 40], "image/png"),
        (HttpStatus::OK, vec![0; 262_145], "image/png"),
    ] {
        let (base, _, task) = server(status, body, content_type).await;
        let provider = TrustWalletIconProvider {
            client: asset_http_client().unwrap(),
            base,
        };
        assert!(provider.resolve_icon(&crypto()).await.unwrap().is_none());
        task.abort();
    }
}
#[tokio::test]
async fn fmp_checks_image_and_normalizes_ticker_without_api_key() {
    let (base, calls, task) = server(HttpStatus::OK, png(), "image/png").await;
    let provider = FmpStockIconProvider {
        client: asset_http_client().unwrap(),
        base,
    };
    let stock = AssetRef::Stock(StockRef {
        ticker: "brk.b".into(),
    });
    assert_eq!(
        provider
            .resolve_icon(&stock)
            .await
            .unwrap()
            .unwrap()
            .as_str(),
        "https://financialmodelingprep.com/image-stock/BRK.B.png"
    );
    assert!(provider.resolve_icon(&crypto()).await.unwrap().is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    task.abort();
    let (base, _, task) = server(HttpStatus::NOT_FOUND, vec![], "image/png").await;
    let provider = FmpStockIconProvider {
        client: asset_http_client().unwrap(),
        base,
    };
    assert!(provider.resolve_icon(&stock).await.unwrap().is_none());
    task.abort();
}
#[tokio::test]
async fn network_failure_is_bounded_and_invalid_paths_never_reach_http() {
    let (base, calls, task) = server(HttpStatus::OK, png(), "image/png").await;
    let provider = FmpStockIconProvider {
        client: asset_http_client().unwrap(),
        base,
    };
    assert!(
        provider
            .resolve_icon(&AssetRef::Stock(StockRef {
                ticker: "AAPL?x=y".into()
            }))
            .await
            .is_err()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    task.abort();
    let provider = TrustWalletIconProvider {
        client: asset_http_client().unwrap(),
        base: "http://127.0.0.1:1".into(),
    };
    assert!(provider.resolve_icon(&crypto()).await.is_err());
}
#[test]
fn evm_addresses_use_the_trust_wallet_checksum_convention() {
    let AssetRef::Crypto(reference) = crypto() else {
        panic!()
    };
    assert_eq!(
        trust_wallet_address(reference.token.as_ref().unwrap()),
        "0xdAC17F958D2ee523a2206206994597C13D831ec7"
    );
}
#[tokio::test]
async fn coingecko_extraction_is_local_and_rejects_ambiguous_symbols() {
    let provider = CoinGeckoIconProvider::default();
    let coin = |id: &str| MarketCoin {
        id: id.into(),
        symbol: "btc".into(),
        name: "Bitcoin".into(),
        price_usd: None,
        image: IconUrl::new("https://coin-images.coingecko.com/btc.png".into()),
    };
    let mut asset = CryptoRef {
        coingecko_id: None,
        symbol: "BTC".into(),
        token: None,
    };
    provider.ingest(&[coin("bitcoin")]);
    assert!(
        provider
            .resolve_icon(&AssetRef::Crypto(asset.clone()))
            .await
            .unwrap()
            .is_some()
    );
    provider.ingest(&[coin("another-btc")]);
    assert!(
        provider
            .resolve_icon(&AssetRef::Crypto(asset.clone()))
            .await
            .unwrap()
            .is_none()
    );
    asset.coingecko_id = Some("bitcoin".into());
    assert!(
        provider
            .resolve_icon(&AssetRef::Crypto(asset))
            .await
            .unwrap()
            .is_some()
    );
    assert!(provider.resolve_icon(&crypto()).await.unwrap().is_none());
}
