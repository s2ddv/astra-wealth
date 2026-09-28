use crate::asset_icon::{
    CoinGeckoIconProvider, RedisAssetCache, TrustWalletIconProvider, asset_http_client,
};
use application::{asset_icon::AssetIconService, market::CryptoMarketService};
use domain::{
    asset_icon::{AssetCache, IconError, IconFuture, IconUrl},
    market::{CryptoMarketProvider, MarketCoin, MarketQuery, validate_coin_id},
};
use reqwest::Client;
use serde::{Deserialize, de::DeserializeOwned};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

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
    price_change_percentage_24h: Option<f64>,
    market_cap: Option<f64>,
    total_volume: Option<f64>,
    market_cap_rank: Option<u32>,
    last_updated: Option<String>,
}
#[derive(Deserialize)]
struct MarketData {
    current_price: Option<UsdPrice>,
    market_cap: Option<UsdPrice>,
    total_volume: Option<UsdPrice>,
    price_change_percentage_24h: Option<f64>,
    last_updated: Option<String>,
}
#[derive(Deserialize)]
struct UsdPrice {
    usd: Option<f64>,
}
impl CoinPayload {
    fn into_coin(self) -> MarketCoin {
        let data = self.market_data;
        MarketCoin {
            id: self.id,
            symbol: self.symbol,
            name: self.name,
            price_usd: self
                .current_price
                .or_else(|| data.as_ref()?.current_price.as_ref()?.usd),
            change_24h: self
                .price_change_percentage_24h
                .or_else(|| data.as_ref()?.price_change_percentage_24h),
            market_cap: self
                .market_cap
                .or_else(|| data.as_ref()?.market_cap.as_ref()?.usd),
            volume_24h: self
                .total_volume
                .or_else(|| data.as_ref()?.total_volume.as_ref()?.usd),
            rank: self.market_cap_rank,
            updated_at: self
                .last_updated
                .or_else(|| data.and_then(|v| v.last_updated)),
            image: self.image.and_then(Image::icon),
        }
    }
}
#[derive(Deserialize)]
struct TrendingPayload {
    coins: Vec<TrendingEntry>,
}
#[derive(Deserialize)]
struct TrendingEntry {
    item: TrendingCoin,
}
#[derive(Deserialize)]
struct TrendingCoin {
    id: String,
    symbol: String,
    name: String,
    large: Option<String>,
    small: Option<String>,
    thumb: Option<String>,
    market_cap_rank: Option<u32>,
}

pub struct CoinGeckoMarketProvider {
    client: Client,
    cache: Arc<dyn AssetCache>,
    icons: Arc<CoinGeckoIconProvider>,
    base: String,
    api_key: Option<String>,
    // Serialize cache fills; recheck cache after acquiring the lock. A short shared
    // cooldown prevents concurrent public requests from hammering a rate-limited upstream.
    gate: tokio::sync::Mutex<Option<Instant>>,
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
            gate: tokio::sync::Mutex::new(None),
        }
    }
    async fn payload<T: DeserializeOwned>(
        &self,
        key: &str,
        path: &str,
        query: &[(&str, &str)],
        ttl: u64,
    ) -> Result<T, IconError> {
        if let Ok(Some(cached)) = self.cache.get(key).await
            && let Ok(value) = serde_json::from_str(&cached)
        {
            return Ok(value);
        }
        let mut cooldown = tokio::time::timeout(Duration::from_secs(10), self.gate.lock())
            .await
            .map_err(|_| IconError::Unavailable)?;
        if let Ok(Some(cached)) = self.cache.get(key).await
            && let Ok(value) = serde_json::from_str(&cached)
        {
            return Ok(value);
        }
        if cooldown.is_some_and(|until| until > Instant::now()) {
            return Err(IconError::RateLimited);
        }
        let mut url =
            url::Url::parse(&format!("{}{path}", self.base)).map_err(|_| IconError::Unavailable)?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let mut request = self.client.get(url);
        if let Some(key) = &self.api_key {
            request = request.header("x-cg-demo-api-key", key);
        }
        let mut response = request.send().await.map_err(|_| IconError::Unavailable)?;
        match response.status() {
            reqwest::StatusCode::OK => {}
            reqwest::StatusCode::NOT_FOUND => return Err(IconError::NotFound),
            reqwest::StatusCode::TOO_MANY_REQUESTS => {
                *cooldown = Some(Instant::now() + Duration::from_secs(60));
                return Err(IconError::RateLimited);
            }
            _ => return Err(IconError::Unavailable),
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| IconError::Unavailable)? {
            if body.len() + chunk.len() > 2_000_000 {
                return Err(IconError::Unavailable);
            }
            body.extend_from_slice(&chunk);
        }
        let json = String::from_utf8(body).map_err(|_| IconError::Unavailable)?;
        let value = serde_json::from_str(&json).map_err(|_| IconError::Unavailable)?;
        let _ = self.cache.set(key, &json, ttl).await;
        Ok(value)
    }
}
impl CryptoMarketProvider for CoinGeckoMarketProvider {
    fn market_page(&self, mut query: MarketQuery) -> IconFuture<'_, Vec<MarketCoin>> {
        Box::pin(async move {
            query.validate_query()?;
            query.ids.sort();
            query.ids.dedup();
            let ids = query.ids.join(",");
            let page = query.page.to_string();
            let per_page = query.per_page.to_string();
            let key = format!("market:coingecko:usd:v2:{page}:{per_page}:{ids}");
            let mut params = vec![
                ("vs_currency", "usd"),
                ("per_page", per_page.as_str()),
                ("page", page.as_str()),
                ("order", "market_cap_desc"),
                ("sparkline", "false"),
            ];
            if !ids.is_empty() {
                params.push(("ids", &ids));
            }
            let payload: Vec<CoinPayload> =
                self.payload(&key, "/coins/markets", &params, 60).await?;
            let coins: Vec<_> = payload.into_iter().map(CoinPayload::into_coin).collect();
            self.icons.ingest(&coins);
            Ok(coins)
        })
    }
    fn trending(&self) -> IconFuture<'_, Vec<MarketCoin>> {
        Box::pin(async move {
            let payload: TrendingPayload = self
                .payload("market:coingecko:trending:v1", "/search/trending", &[], 300)
                .await?;
            let coins: Vec<_> = payload
                .coins
                .into_iter()
                .take(15)
                .map(|entry| {
                    let coin = entry.item;
                    MarketCoin {
                        id: coin.id,
                        symbol: coin.symbol,
                        name: coin.name,
                        rank: coin.market_cap_rank,
                        image: [coin.large, coin.small, coin.thumb]
                            .into_iter()
                            .flatten()
                            .find_map(IconUrl::new),
                        ..MarketCoin::default()
                    }
                })
                .collect();
            self.icons.ingest(&coins);
            Ok(coins)
        })
    }
    fn coin<'a>(&'a self, id: &'a str) -> IconFuture<'a, MarketCoin> {
        Box::pin(async move {
            validate_coin_id(id)?;
            let key = format!("market:coingecko:coin:{id}:v2");
            let payload: CoinPayload = self
                .payload(
                    &key,
                    &format!("/coins/{id}"),
                    &[
                        ("localization", "false"),
                        ("tickers", "false"),
                        ("community_data", "false"),
                        ("developer_data", "false"),
                    ],
                    60,
                )
                .await?;
            let coin = payload.into_coin();
            if coin.id != id {
                return Err(IconError::Unavailable);
            }
            self.icons.ingest(std::slice::from_ref(&coin));
            Ok(coin)
        })
    }
}

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
