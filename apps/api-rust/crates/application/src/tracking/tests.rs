use super::*;
use domain::wallet::*;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[test]
fn raw_balances_are_exact_at_all_evm_scales() {
    for (raw, decimals, expected) in [
        ("0x0", 18, "0"),
        ("0x1", 0, "1"),
        ("0x1", 6, "0.000001"),
        ("0xde0b6b3a7640001", 18, "1.000000000000000001"),
        ("0x1", 24, "0.000000000000000000000001"),
    ] {
        assert_eq!(hex_amount(raw, decimals).unwrap(), expected);
    }
    assert_eq!(
        hex_amount(&format!("0x{}", "f".repeat(64)), 0).unwrap(),
        "115792089237316195423570985008687907853269984665640564039457584007913129639935"
    );
    assert_eq!(
        hex_amount("0x1", 255).unwrap(),
        format!("0.{}1", "0".repeat(254))
    );
    for raw in [
        "",
        "0x",
        "0x-1",
        "0xno",
        "123",
        &format!("0x{}", "f".repeat(65)),
    ] {
        assert!(hex_amount(raw, 18).is_err());
    }
    assert!(price_decimal("1e9999999").is_err());
    assert!(price_decimal("-1").is_err());
}
struct Wallets {
    chain: Chain,
}
impl WalletRepository for Wallets {
    fn find_by_id<'a>(&'a self, id: &'a str, user: &'a str) -> WalletFuture<'a, Option<Wallet>> {
        Box::pin(async move {
            Ok((id == "wallet" && user == "owner").then(|| Wallet {
                id: id.into(),
                user_id: user.into(),
                chain: self.chain,
                address: "0x0000000000000000000000000000000000000001".into(),
                nickname: None,
                created_at: chrono::Utc::now().naive_utc(),
                updated_at: chrono::Utc::now().naive_utc(),
                last_synced_at: None,
                assets: vec![],
            }))
        })
    }
    fn create<'a>(&'a self, _: &'a str, _: NewWallet) -> WalletFuture<'a, Wallet> {
        unreachable!()
    }
    fn find_by_user_id<'a>(&'a self, _: &'a str) -> WalletFuture<'a, Vec<Wallet>> {
        unreachable!()
    }
    fn update_nickname<'a>(&'a self, _: &'a str, _: &'a str, _: &'a str) -> WalletFuture<'a, u64> {
        unreachable!()
    }
    fn delete<'a>(&'a self, _: &'a str, _: &'a str) -> WalletFuture<'a, u64> {
        unreachable!()
    }
}
#[derive(Default)]
struct ChainMock {
    calls: AtomicUsize,
    zero: AtomicBool,
    failed: AtomicBool,
}
impl OnChainProvider for ChainMock {
    fn native_balance<'a>(&'a self, _: Chain, _: &'a str) -> TrackingFuture<'a, String> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.failed.load(Ordering::SeqCst) {
                Err(TrackingError::ProviderUnavailable)
            } else {
                Ok("0xde0b6b3a7640001".into())
            }
        })
    }
    fn token_balances<'a>(&'a self, _: Chain, _: &'a str) -> TrackingFuture<'a, TokenBalances> {
        Box::pin(async move {
            Ok(TokenBalances {
                complete: true,
                tokens: vec![
                    TokenBalance {
                        contract: "token".into(),
                        raw: if self.zero.load(Ordering::SeqCst) {
                            "0x0"
                        } else {
                            "0xf4240"
                        }
                        .into(),
                    },
                    TokenBalance {
                        contract: "no-meta".into(),
                        raw: "0x1".into(),
                    },
                ],
            })
        })
    }
    fn token_metadata<'a>(
        &'a self,
        _: Chain,
        contract: &'a str,
    ) -> TrackingFuture<'a, Option<TokenMetadata>> {
        Box::pin(async move {
            Ok((contract != "no-meta").then(|| TokenMetadata {
                symbol: "UNKNOWN".into(),
                name: "Unknown token".into(),
                decimals: 6,
            }))
        })
    }
}
struct Prices {
    missing: bool,
}
impl PriceProvider for Prices {
    fn native_price(&self, _: Chain) -> TrackingFuture<'_, Option<String>> {
        Box::pin(async move { Ok((!self.missing).then(|| "2000.123456789".into())) })
    }
    fn token_prices<'a>(
        &'a self,
        _: Chain,
        _: &'a [String],
    ) -> TrackingFuture<'a, HashMap<String, String>> {
        Box::pin(async { Ok(HashMap::new()) })
    }
}
#[derive(Default)]
struct Repo(Mutex<Vec<TrackedAsset>>);
impl TrackingRepository for Repo {
    fn save<'a>(
        &'a self,
        user: &'a str,
        w: &'a TrackedWallet,
        observed: &'a [String],
        complete: bool,
    ) -> TrackingFuture<'a, ()> {
        Box::pin(async move {
            assert_eq!(user, "owner");
            // Missing metadata makes this snapshot partial. It must not prune
            // a previously persisted nonzero balance for that contract.
            assert!(!complete);
            assert!(!w.complete);
            assert!(!w.valuation_complete);
            assert!(!observed.contains(&"no-meta".into()));
            assert!(observed.contains(&"token".into()));
            *self.0.lock().unwrap() = w.assets.clone();
            Ok(())
        })
    }
}
#[derive(Default)]
struct Cache {
    value: Mutex<Option<TrackedWallet>>,
    lock: AtomicBool,
    synced: AtomicBool,
    calls: AtomicUsize,
}
impl TrackingCache for Cache {
    fn get<'a>(&'a self, _: &'a str) -> TrackingFuture<'a, Option<TrackedWallet>> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.value.lock().unwrap().clone())
        })
    }
    fn put<'a>(&'a self, _: &'a str, w: &'a TrackedWallet) -> TrackingFuture<'a, ()> {
        Box::pin(async move {
            *self.value.lock().unwrap() = Some(w.clone());
            Ok(())
        })
    }
    fn invalidate<'a>(&'a self, _: &'a str) -> TrackingFuture<'a, ()> {
        Box::pin(async move {
            *self.value.lock().unwrap() = None;
            Ok(())
        })
    }
    fn acquire<'a>(&'a self, _: &'a str) -> TrackingFuture<'a, Option<String>> {
        Box::pin(
            async move { Ok((!self.lock.swap(true, Ordering::SeqCst)).then(|| "lease".into())) },
        )
    }
    fn release<'a>(&'a self, _: &'a str, _: &'a str) -> TrackingFuture<'a, ()> {
        Box::pin(async move {
            self.lock.store(false, Ordering::SeqCst);
            Ok(())
        })
    }
    fn allow_sync<'a>(&'a self, _: &'a str) -> TrackingFuture<'a, bool> {
        Box::pin(async move { Ok(!self.synced.swap(true, Ordering::SeqCst)) })
    }
}
struct Fixture {
    service: WalletTrackingService,
    chain: Arc<ChainMock>,
    repo: Arc<Repo>,
    cache: Arc<Cache>,
}
fn fixture(chain: Chain, missing: bool) -> Fixture {
    let provider = Arc::new(ChainMock::default());
    let repo = Arc::new(Repo::default());
    let cache = Arc::new(Cache::default());
    Fixture {
        service: WalletTrackingService::new(
            Arc::new(Wallets { chain }),
            provider.clone(),
            Arc::new(Prices { missing }),
            repo.clone(),
            cache.clone(),
        ),
        chain: provider,
        repo,
        cache,
    }
}
#[tokio::test]
async fn authorization_and_supported_chain_precede_cache_and_provider() {
    let f = fixture(Chain::Ethereum, false);
    assert!(matches!(
        f.service.assets("other", "wallet", false).await,
        Err(TrackingError::NotFound)
    ));
    assert_eq!(f.chain.calls.load(Ordering::SeqCst), 0);
    assert_eq!(f.cache.calls.load(Ordering::SeqCst), 0);
    for chain in [Chain::Solana, Chain::Bitcoin] {
        let f = fixture(chain, false);
        assert!(matches!(
            f.service.assets("owner", "wallet", false).await,
            Err(TrackingError::ChainNotSupported)
        ));
        assert_eq!(f.chain.calls.load(Ordering::SeqCst), 0);
    }
}
#[tokio::test]
async fn value_cache_sync_and_zero_pruning() {
    let f = fixture(Chain::Ethereum, false);
    let first = f.service.assets("owner", "wallet", false).await.unwrap();
    assert_eq!(
        first.total_usd.as_deref(),
        Some("2000.123456789000002000123456789")
    );
    assert_eq!(first.assets.len(), 2);
    assert!(first.assets[0].is_native);
    assert_eq!(first.assets[1].amount, "1");
    assert_eq!(first.assets[1].price_usd, None);
    assert_eq!(first.assets[1].value_usd, None);
    f.service.assets("owner", "wallet", false).await.unwrap();
    assert_eq!(f.chain.calls.load(Ordering::SeqCst), 1);
    // Cache does not bypass authorization.
    assert!(matches!(
        f.service.assets("other", "wallet", false).await,
        Err(TrackingError::NotFound)
    ));
    f.chain.zero.store(true, Ordering::SeqCst);
    let updated = f.service.assets("owner", "wallet", true).await.unwrap();
    assert_eq!(updated.assets.len(), 1);
    assert_eq!(f.repo.0.lock().unwrap().len(), 1);
    assert!(matches!(
        f.service.assets("owner", "wallet", true).await,
        Err(TrackingError::RateLimited)
    ));
    assert!(!f.cache.lock.load(Ordering::SeqCst));
}
#[tokio::test]
async fn missing_prices_and_upstream_failure() {
    let f = fixture(Chain::Ethereum, true);
    assert_eq!(
        f.service
            .assets("owner", "wallet", false)
            .await
            .unwrap()
            .total_usd,
        None
    );
    f.chain.failed.store(true, Ordering::SeqCst);
    assert!(matches!(
        f.service.assets("owner", "wallet", true).await,
        Err(TrackingError::ProviderUnavailable)
    ));
    assert!(!f.cache.lock.load(Ordering::SeqCst));
    assert_eq!(f.repo.0.lock().unwrap().len(), 2); // Failed sync does not overwrite holdings.
}
