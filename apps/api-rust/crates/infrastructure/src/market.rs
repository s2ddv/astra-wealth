use crate::asset_icon::{
    CoinGeckoIconProvider, RedisAssetCache, TrustWalletIconProvider, asset_http_client,
};
use application::{asset_icon::AssetIconService, market::CryptoMarketService};
use domain::{
    asset_icon::{AssetCache, AssetRef, CryptoRef, IconError, IconFuture, IconUrl},
    market::{CryptoMarketProvider, MarketCoin},
};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(untagged)]
enum Image {
    Url(String),
    Sizes {
        large: Option<String>,
        small: Option<String>,
        thumb: Option<String>,
    },
}
impl Image {
    fn icon(self) -> Option<IconUrl> {
        match self {
            Self::Url(url) => IconUrl::new(url),
            Self::Sizes {
                large,
                small,
                thumb,
            } => [large, small, thumb]
                .into_iter()
                .flatten()
                .find_map(IconUrl::new),
        }
    }
}
#[derive(Deserialize)]
struct CoinPayload {
    id: String,
    symbol: String,
    name: String,
    image: Option<Image>,
    current_price: Option<f64>,
    market_data: Option<MarketData>,
}
#[derive(Deserialize)]
struct MarketData {
    current_price: Option<UsdPrice>,
}
#[derive(Deserialize)]
struct UsdPrice {
    usd: Option<f64>,
}
impl CoinPayload {
    fn into_coin(self) -> MarketCoin {
        MarketCoin {
            id: self.id,
            symbol: self.symbol,
            name: self.name,
            price_usd: self.current_price.or_else(|| {
                self.market_data
                    .and_then(|data| data.current_price)
                    .and_then(|price| price.usd)
            }),
            image: self.image.and_then(Image::icon),
        }
    }
}

pub struct CoinGeckoMarketProvider {
    client: Client,
    cache: Arc<dyn AssetCache>,
    icons: Arc<CoinGeckoIconProvider>,
    base: String,
    api_key: Option<String>,
}
impl CoinGeckoMarketProvider {
    pub fn new(
        client: Client,
        cache: Arc<dyn AssetCache>,
        icons: Arc<CoinGeckoIconProvider>,
        api_key: Option<String>,
    ) -> Self {
        Self {
            client,
            cache,
            icons,
            base: "https://api.coingecko.com/api/v3".into(),
            api_key,
        }
    }
    async fn payload(
        &self,
        key: &str,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<(String, bool), IconError> {
        if let Ok(Some(cached)) = self.cache.get(key).await {
            return Ok((cached, false));
        }
        let mut url =
            url::Url::parse(&format!("{}{path}", self.base)).map_err(|_| IconError::Unavailable)?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let mut request = self.client.get(url);
        if let Some(key) = &self.api_key {
            request = request.header("x-cg-demo-api-key", key);
        }
        let mut response = request.send().await.map_err(|_| IconError::Unavailable)?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(IconError::Unavailable);
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| IconError::Unavailable)? {
            if body.len() + chunk.len() > 2_000_000 {
                return Err(IconError::Unavailable);
            }
            body.extend_from_slice(&chunk);
        }
        String::from_utf8(body)
            .map(|json| (json, true))
            .map_err(|_| IconError::Unavailable)
    }
}
impl CryptoMarketProvider for CoinGeckoMarketProvider {
    fn markets(&self) -> IconFuture<'_, Vec<MarketCoin>> {
        Box::pin(async move {
            let key = "market:coingecko:usd:top100:v1";
            let (json, fresh) = self
                .payload(
                    key,
                    "/coins/markets",
                    &[
                        ("vs_currency", "usd"),
                        ("per_page", "100"),
                        ("page", "1"),
                        ("sparkline", "false"),
                    ],
                )
                .await?;
            let payload: Vec<CoinPayload> =
                serde_json::from_str(&json).map_err(|_| IconError::Unavailable)?;
            let coins: Vec<_> = payload.into_iter().map(CoinPayload::into_coin).collect();
            self.icons.ingest(&coins);
            if fresh {
                let _ = self.cache.set(key, &json, 60).await;
            }
            Ok(coins)
        })
    }
    fn coin<'a>(&'a self, id: &'a str) -> IconFuture<'a, MarketCoin> {
        Box::pin(async move {
            AssetRef::Crypto(CryptoRef {
                coingecko_id: Some(id.to_owned()),
                symbol: "coin".into(),
                token: None,
            })
            .validate()?;
            let key = format!("market:coingecko:coin:{id}:v1");
            let (json, fresh) = self
                .payload(
                    &key,
                    &format!("/coins/{id}"),
                    &[
                        ("localization", "false"),
                        ("tickers", "false"),
                        ("community_data", "false"),
                        ("developer_data", "false"),
                    ],
                )
                .await?;
            let coin = serde_json::from_str::<CoinPayload>(&json)
                .map_err(|_| IconError::Unavailable)?
                .into_coin();
            if coin.id != id {
                return Err(IconError::Unavailable);
            }
            self.icons.ingest(std::slice::from_ref(&coin));
            if fresh {
                let _ = self.cache.set(&key, &json, 60).await;
            }
            Ok(coin)
        })
    }
}

/// Shared pool and HTTP client for market ingestion and crypto icons. FMP is absent.
pub fn crypto_market_services(
    pool: deadpool_redis::Pool,
    api_key: Option<String>,
) -> Result<(CryptoMarketService, AssetIconService), reqwest::Error> {
    let client = asset_http_client()?;
    let cache = Arc::new(RedisAssetCache::new(pool));
    let primary = Arc::new(CoinGeckoIconProvider::default());
    let icons = AssetIconService::crypto(
        primary.clone(),
        Arc::new(TrustWalletIconProvider::new(client.clone())),
        cache.clone(),
    );
    let market = Arc::new(CoinGeckoMarketProvider::new(
        client, cache, primary, api_key,
    ));
    Ok((CryptoMarketService::new(market, icons.clone()), icons))
}

#[cfg(test)]
mod tests;
