use domain::asset_icon::{
    AssetCache, AssetIconProvider, AssetRef, IconError, IconResponse, IconUrl, StockIconProvider,
};
use std::sync::Arc;

pub const ICON_TTL_SECONDS: u64 = 24 * 60 * 60;
#[derive(Clone)]
pub struct AssetIconService {
    primary: Arc<dyn AssetIconProvider>,
    fallback: Arc<dyn AssetIconProvider>,
    stocks: Option<Arc<dyn StockIconProvider>>,
    cache: Arc<dyn AssetCache>,
}
impl AssetIconService {
    pub fn crypto(
        primary: Arc<dyn AssetIconProvider>,
        fallback: Arc<dyn AssetIconProvider>,
        cache: Arc<dyn AssetCache>,
    ) -> Self {
        Self {
            primary,
            fallback,
            cache,
            stocks: None,
        }
    }
    /// Explicit opt-in for future stock consumers; production crypto wiring never calls this.
    pub fn with_stocks(mut self, stocks: Arc<dyn StockIconProvider>) -> Self {
        self.stocks = Some(stocks);
        self
    }

    pub async fn resolve_icon(&self, asset: &AssetRef) -> Result<IconResponse, IconError> {
        asset.validate()?;
        let key = asset.cache_key();
        // Extraction is local and free. Prefer a fresh primary over a cached fallback.
        let primary = match asset {
            AssetRef::Crypto(_) => self.primary.resolve_icon(asset).await.ok().flatten(),
            AssetRef::Stock(_) => None,
        };
        if let Some(url) = primary {
            let _ = self.cache.set(&key, url.as_str(), ICON_TTL_SECONDS).await;
            return Ok(IconResponse {
                url: Some(url),
                symbol: asset.symbol(),
            });
        }
        // Without explicit stock opt-in, even an existing cache entry must not enable stocks.
        let enabled = !matches!(asset, AssetRef::Stock(_)) || self.stocks.is_some();
        if enabled
            && !asset.is_symbol_only()
            && let Ok(Some(value)) = self.cache.get(&key).await
            && let Some(url) = IconUrl::new(value)
        {
            return Ok(IconResponse {
                url: Some(url),
                symbol: asset.symbol(),
            });
        }
        let url = match asset {
            AssetRef::Crypto(_) => self.fallback.resolve_icon(asset).await.ok().flatten(),
            AssetRef::Stock(_) => match &self.stocks {
                Some(provider) => provider.resolve_icon(asset).await.ok().flatten(),
                None => None,
            },
        };
        if let Some(url) = &url {
            let _ = self.cache.set(&key, url.as_str(), ICON_TTL_SECONDS).await;
        }
        // No network request, negative 24h cache, or fabricated image for placeholders.
        Ok(IconResponse {
            url,
            symbol: asset.symbol(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::asset_icon::{CryptoRef, IconFuture, StockRef, TokenChain, TokenRef};
    use std::sync::Mutex;

    struct Provider {
        label: &'static str,
        value: Option<IconUrl>,
        fail: bool,
        calls: Arc<Mutex<Vec<String>>>,
    }
    impl AssetIconProvider for Provider {
        fn resolve_icon<'a>(&'a self, _: &'a AssetRef) -> IconFuture<'a, Option<IconUrl>> {
            Box::pin(async {
                self.calls.lock().unwrap().push(self.label.into());
                if self.fail {
                    Err(IconError::Unavailable)
                } else {
                    Ok(self.value.clone())
                }
            })
        }
    }
    impl StockIconProvider for Provider {}
    struct Cache {
        value: Mutex<Option<String>>,
        fail: bool,
        calls: Arc<Mutex<Vec<String>>>,
    }
    impl AssetCache for Cache {
        fn get<'a>(&'a self, key: &'a str) -> IconFuture<'a, Option<String>> {
            Box::pin(async move {
                self.calls.lock().unwrap().push(format!("get:{key}"));
                if self.fail {
                    Err(IconError::Unavailable)
                } else {
                    Ok(self.value.lock().unwrap().clone())
                }
            })
        }
        fn set<'a>(&'a self, key: &'a str, value: &'a str, ttl: u64) -> IconFuture<'a, ()> {
            Box::pin(async move {
                self.calls.lock().unwrap().push(format!("set:{key}:{ttl}"));
                if self.fail {
                    Err(IconError::Unavailable)
                } else {
                    *self.value.lock().unwrap() = Some(value.into());
                    Ok(())
                }
            })
        }
    }
    fn url() -> IconUrl {
        IconUrl::new("https://coin-images.coingecko.com/coins/btc.png".into()).unwrap()
    }
    fn asset() -> AssetRef {
        AssetRef::Crypto(CryptoRef {
            coingecko_id: Some("bitcoin".into()),
            symbol: "btc".into(),
            token: None,
        })
    }
    fn setup(
        primary: Option<IconUrl>,
        fallback: Option<IconUrl>,
        cached: Option<String>,
        fail: bool,
    ) -> (AssetIconService, Arc<Mutex<Vec<String>>>) {
        let calls = Arc::new(Mutex::new(vec![]));
        let service = AssetIconService::crypto(
            Arc::new(Provider {
                label: "primary",
                value: primary,
                fail,
                calls: calls.clone(),
            }),
            Arc::new(Provider {
                label: "fallback",
                value: fallback,
                fail: false,
                calls: calls.clone(),
            }),
            Arc::new(Cache {
                value: Mutex::new(cached),
                fail,
                calls: calls.clone(),
            }),
        );
        (service, calls)
    }
    #[tokio::test]
    async fn primary_short_circuits_and_caches_for_24_hours() {
        let (service, calls) = setup(Some(url()), None, None, false);
        assert_eq!(
            service.resolve_icon(&asset()).await.unwrap().url,
            Some(url())
        );
        assert_eq!(
            *calls.lock().unwrap(),
            ["primary", "set:icon:crypto:id:bitcoin:86400"]
        );
    }
    #[tokio::test]
    async fn cache_hit_skips_network_fallback() {
        let (service, calls) = setup(None, None, Some(url().as_str().into()), false);
        assert_eq!(
            service.resolve_icon(&asset()).await.unwrap().url,
            Some(url())
        );
        assert_eq!(
            *calls.lock().unwrap(),
            ["primary", "get:icon:crypto:id:bitcoin"]
        );
    }
    #[tokio::test]
    async fn fallback_order_and_placeholder_without_negative_cache() {
        let (service, calls) = setup(None, Some(url()), None, false);
        assert!(service.resolve_icon(&asset()).await.unwrap().url.is_some());
        assert_eq!(
            *calls.lock().unwrap(),
            [
                "primary",
                "get:icon:crypto:id:bitcoin",
                "fallback",
                "set:icon:crypto:id:bitcoin:86400"
            ]
        );
        let (service, calls) = setup(None, None, None, false);
        let result = service.resolve_icon(&asset()).await.unwrap();
        assert_eq!(
            result,
            IconResponse {
                url: None,
                symbol: "BTC".into()
            }
        );
        assert_eq!(calls.lock().unwrap().len(), 3);
    }
    #[tokio::test]
    async fn dependency_failures_and_invalid_cached_urls_do_not_break_resolution() {
        let (service, _) = setup(None, Some(url()), None, true);
        assert_eq!(
            service.resolve_icon(&asset()).await.unwrap().url,
            Some(url())
        );
        let (service, _) = setup(
            None,
            None,
            Some("https://raw.githubusercontent.com/unsafe".into()),
            false,
        );
        assert!(service.resolve_icon(&asset()).await.unwrap().url.is_none());
    }
    #[tokio::test]
    async fn validation_happens_before_any_io() {
        let (service, calls) = setup(None, None, None, false);
        let invalid = AssetRef::Stock(StockRef {
            ticker: "../../secret".into(),
        });
        assert!(matches!(
            service.resolve_icon(&invalid).await,
            Err(IconError::InvalidReference)
        ));
        assert!(calls.lock().unwrap().is_empty());
    }
    #[tokio::test]
    async fn stocks_are_disabled_until_explicitly_injected() {
        let (service, calls) = setup(Some(url()), None, Some(url().as_str().into()), false);
        let asset = AssetRef::Stock(StockRef {
            ticker: "aapl".into(),
        });
        assert!(service.resolve_icon(&asset).await.unwrap().url.is_none());
        assert!(calls.lock().unwrap().is_empty());
        let (service, calls) = setup(None, None, None, false);
        let service = service.with_stocks(Arc::new(Provider {
            label: "stocks",
            value: Some(url()),
            fail: false,
            calls: calls.clone(),
        }));
        assert!(service.resolve_icon(&asset).await.unwrap().url.is_some());
        assert_eq!(
            *calls.lock().unwrap(),
            ["get:icon:stock:AAPL", "stocks", "set:icon:stock:AAPL:86400"]
        );
    }
    #[tokio::test]
    async fn ambiguous_symbol_does_not_reuse_a_stale_cached_icon() {
        let (service, calls) = setup(None, None, Some(url().as_str().into()), false);
        let asset = AssetRef::Crypto(CryptoRef {
            coingecko_id: None,
            symbol: "btc".into(),
            token: None,
        });
        assert!(service.resolve_icon(&asset).await.unwrap().url.is_none());
        assert_eq!(*calls.lock().unwrap(), ["primary", "fallback"]);
    }
    #[test]
    fn cache_keys_separate_asset_classes_chains_and_case_sensitive_addresses() {
        let token = |chain, contract: &str| {
            AssetRef::Crypto(CryptoRef {
                coingecko_id: None,
                symbol: "USDT".into(),
                token: Some(TokenRef {
                    chain,
                    contract: contract.into(),
                }),
            })
        };
        assert_ne!(
            token(TokenChain::Ethereum, "0xab").cache_key(),
            token(TokenChain::Base, "0xab").cache_key()
        );
        assert_ne!(
            token(TokenChain::Solana, "Ab").cache_key(),
            token(TokenChain::Solana, "ab").cache_key()
        );
        assert_eq!(
            token(TokenChain::Ethereum, "0xAB").cache_key(),
            token(TokenChain::Ethereum, "0xab").cache_key()
        );
    }
}
