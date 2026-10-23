use application::public_wallet::{NativeBalanceProvider, PublicWalletService, display_wei};
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use domain::asset_icon::{AssetCache, IconError, IconFuture};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tower::ServiceExt;
#[derive(Default)]
struct Cache {
    values: Mutex<HashMap<String, String>>,
    fail: bool,
}
impl AssetCache for Cache {
    fn get<'a>(&'a self, key: &'a str) -> IconFuture<'a, Option<String>> {
        Box::pin(async move {
            if self.fail {
                Err(IconError::Unavailable)
            } else {
                Ok(self.values.lock().unwrap().get(key).cloned())
            }
        })
    }
    fn set<'a>(&'a self, key: &'a str, value: &'a str, ttl: u64) -> IconFuture<'a, ()> {
        assert_eq!(ttl, 30);
        Box::pin(async move {
            self.values.lock().unwrap().insert(key.into(), value.into());
            Ok(())
        })
    }
}
struct Provider {
    fail: bool,
    calls: Arc<AtomicUsize>,
}
impl NativeBalanceProvider for Provider {
    fn balance<'a>(&'a self, _: &'a str) -> IconFuture<'a, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            if self.fail {
                Err(IconError::Unavailable)
            } else {
                Ok("1.234568".into())
            }
        })
    }
}
async fn get(app: axum::Router, address: &str) -> (u16, serde_json::Value) {
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/v1/wallet/{address}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    (
        status,
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap(),
    )
}
#[tokio::test]
async fn public_contract_validation_partial_failure_and_case_insensitive_cache() {
    let calls = Arc::new(AtomicUsize::new(0));
    let app = astra_api::public_wallet::router(PublicWalletService::new(
        Arc::new(Provider {
            fail: false,
            calls: calls.clone(),
        }),
        Arc::new(Provider {
            fail: true,
            calls: calls.clone(),
        }),
        Arc::new(Cache::default()),
    ));
    assert_eq!(
        get(app.clone(), "invalid").await,
        (400, serde_json::json!({"error":"Invalid Ethereum address"}))
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let (status, summary) = get(app.clone(), "0xAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").await;
    assert_eq!(status, 200);
    assert_eq!(summary["alchemy"]["nativeBalance"], "1.234568");
    assert_eq!(summary["alchemy"]["source"], "alchemy");
    assert!(summary["moralis"].is_null());
    assert_eq!(summary.as_object().unwrap().len(), 4);
    chrono::DateTime::parse_from_rfc3339(summary["fetchedAt"].as_str().unwrap()).unwrap();
    assert_eq!(
        get(app, "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            .await
            .1,
        summary
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
#[tokio::test]
async fn two_failed_sources_still_return_summary_but_failed_cache_returns_502() {
    for fail_cache in [false, true] {
        let calls = Arc::new(AtomicUsize::new(0));
        let provider = Arc::new(Provider {
            fail: true,
            calls: calls.clone(),
        });
        let app = astra_api::public_wallet::router(PublicWalletService::new(
            provider.clone(),
            provider,
            Arc::new(Cache {
                fail: fail_cache,
                ..Default::default()
            }),
        ));
        let (status, body) = get(app, "0x0000000000000000000000000000000000000000").await;
        if fail_cache {
            assert_eq!(status, 502);
            assert_eq!(calls.load(Ordering::SeqCst), 0);
        } else {
            assert_eq!(status, 200);
            assert!(body["alchemy"].is_null() && body["moralis"].is_null());
        }
    }
}
#[test]
fn wei_conversion_preserves_six_decimal_display_without_float_loss() {
    for (input, radix, expected) in [
        ("0x0", 16, "0.000000"),
        ("1000000000000000000", 10, "1.000000"),
        ("1234567500000000000", 10, "1.234568"),
        ("0xde0b6b3a7640000", 16, "1.000000"),
    ] {
        assert_eq!(display_wei(input, radix).unwrap(), expected);
    }
    for input in ["-1", "1.5", "NaN", ""] {
        assert!(display_wei(input, 10).is_err());
    }
}
