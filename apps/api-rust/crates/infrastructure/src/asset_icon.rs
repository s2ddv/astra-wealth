//! Shared HTTP/cache adapters. Trust Wallet bytes are cached and returned as a PNG
//! data URL: the browser never contacts raw.githubusercontent.com.
use base64::{Engine, engine::general_purpose::STANDARD};
use domain::asset_icon::{
    AssetCache, AssetIconProvider, AssetRef, IconError, IconFuture, IconUrl, StockIconProvider,
};
use reqwest::{Client, StatusCode};
use std::{
    collections::HashMap,
    sync::RwLock,
    time::{Duration, Instant},
};

pub fn asset_http_client() -> Result<Client, reqwest::Error> {
    Client::builder()
        .timeout(Duration::from_secs(5))
        .connect_timeout(Duration::from_secs(2))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("astra-wealth/asset-service")
        .build()
}

pub struct RedisAssetCache {
    pool: deadpool_redis::Pool,
}
impl RedisAssetCache {
    pub fn new(pool: deadpool_redis::Pool) -> Self {
        Self { pool }
    }
}
impl AssetCache for RedisAssetCache {
    fn get<'a>(&'a self, key: &'a str) -> IconFuture<'a, Option<String>> {
        Box::pin(async move {
            tokio::time::timeout(Duration::from_secs(1), async {
                let mut conn = self.pool.get().await.map_err(|_| IconError::Unavailable)?;
                deadpool_redis::redis::cmd("GET")
                    .arg(key)
                    .query_async(&mut conn)
                    .await
                    .map_err(|_| IconError::Unavailable)
            })
            .await
            .map_err(|_| IconError::Unavailable)?
        })
    }
    fn set<'a>(&'a self, key: &'a str, value: &'a str, ttl: u64) -> IconFuture<'a, ()> {
        Box::pin(async move {
            tokio::time::timeout(Duration::from_secs(1), async {
                let mut conn = self.pool.get().await.map_err(|_| IconError::Unavailable)?;
                deadpool_redis::redis::cmd("SETEX")
                    .arg(key)
                    .arg(ttl)
                    .arg(value)
                    .query_async(&mut conn)
                    .await
                    .map_err(|_| IconError::Unavailable)
            })
            .await
            .map_err(|_| IconError::Unavailable)?
        })
    }
}

#[derive(Clone)]
struct CoinIcon {
    id: String,
    symbol: String,
    image: Option<IconUrl>,
    seen: Instant,
}
#[derive(Default)]
pub struct CoinGeckoIconProvider {
    coins: RwLock<HashMap<String, CoinIcon>>,
}
impl CoinGeckoIconProvider {
    /// Called by the market adapter after decoding its existing response, never by HTTP handlers.
    pub fn ingest(&self, coins: &[domain::market::MarketCoin]) {
        let Ok(mut entries) = self.coins.write() else {
            return;
        };
        entries.retain(|_, coin| coin.seen.elapsed() < Duration::from_secs(86_400));
        // Bound process-local metadata independently of Redis.
        if entries.len() + coins.len() > 10_000 {
            entries.clear();
        }
        for coin in coins.iter().take(10_000) {
            entries.insert(
                coin.id.clone(),
                CoinIcon {
                    id: coin.id.clone(),
                    symbol: coin.symbol.to_ascii_uppercase(),
                    image: coin.image.clone(),
                    seen: Instant::now(),
                },
            );
        }
    }
}
impl AssetIconProvider for CoinGeckoIconProvider {
    fn resolve_icon<'a>(&'a self, asset: &'a AssetRef) -> IconFuture<'a, Option<IconUrl>> {
        Box::pin(async move {
            asset.validate()?;
            let AssetRef::Crypto(asset) = asset else {
                return Ok(None);
            };
            // A contract must not accidentally resolve to a different token with the same ticker.
            if asset.coingecko_id.is_none() && asset.token.is_some() {
                return Ok(None);
            }
            let entries = self.coins.read().map_err(|_| IconError::Unavailable)?;
            let mut matches = entries.values().filter(|coin| {
                coin.seen.elapsed() < Duration::from_secs(86_400)
                    && match &asset.coingecko_id {
                        Some(id) => coin.id == *id,
                        None => coin.symbol.eq_ignore_ascii_case(&asset.symbol),
                    }
            });
            let first = matches.next();
            if matches.next().is_some() {
                return Ok(None);
            }
            Ok(first.and_then(|coin| coin.image.clone()))
        })
    }
}

pub struct TrustWalletIconProvider {
    client: Client,
    base: String,
}
impl TrustWalletIconProvider {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            base: "https://raw.githubusercontent.com/trustwallet/assets/master/blockchains".into(),
        }
    }
}
impl AssetIconProvider for TrustWalletIconProvider {
    fn resolve_icon<'a>(&'a self, asset: &'a AssetRef) -> IconFuture<'a, Option<IconUrl>> {
        Box::pin(async move {
            asset.validate()?;
            let AssetRef::Crypto(reference) = asset else {
                return Ok(None);
            };
            let Some(token) = &reference.token else {
                return Ok(None);
            };
            let address = trust_wallet_address(token);
            let url = format!(
                "{}/{}/assets/{address}/logo.png",
                self.base,
                token.chain.as_str()
            );
            let Some(bytes) = download_png(&self.client, &url).await? else {
                return Ok(None);
            };
            Ok(IconUrl::new(format!(
                "data:image/png;base64,{}",
                STANDARD.encode(bytes)
            )))
        })
    }
}

pub struct FmpStockIconProvider {
    client: Client,
    base: String,
}
impl FmpStockIconProvider {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            base: "https://financialmodelingprep.com/image-stock".into(),
        }
    }
}
impl StockIconProvider for FmpStockIconProvider {}
impl AssetIconProvider for FmpStockIconProvider {
    fn resolve_icon<'a>(&'a self, asset: &'a AssetRef) -> IconFuture<'a, Option<IconUrl>> {
        Box::pin(async move {
            asset.validate()?;
            let AssetRef::Stock(stock) = asset else {
                return Ok(None);
            };
            let url = format!("{}/{}.png", self.base, stock.ticker.to_ascii_uppercase());
            // Download once to detect upstream errors masquerading as successful HTML.
            if download_png(&self.client, &url).await?.is_none() {
                return Ok(None);
            }
            Ok(IconUrl::new(format!(
                "https://financialmodelingprep.com/image-stock/{}.png",
                stock.ticker.to_ascii_uppercase()
            )))
        })
    }
}

fn trust_wallet_address(token: &domain::asset_icon::TokenRef) -> String {
    use sha3::{Digest, Keccak256};
    if token.chain == domain::asset_icon::TokenChain::Solana {
        return token.contract.clone();
    }
    let lower = token.contract[2..].to_ascii_lowercase();
    let hash = Keccak256::digest(lower.as_bytes());
    let mut address = String::from("0x");
    for (index, c) in lower.bytes().enumerate() {
        let nibble = if index % 2 == 0 {
            hash[index / 2] >> 4
        } else {
            hash[index / 2] & 15
        };
        address.push(if nibble >= 8 {
            c.to_ascii_uppercase()
        } else {
            c
        } as char);
    }
    address
}

async fn download_png(client: &Client, url: &str) -> Result<Option<Vec<u8>>, IconError> {
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|_| IconError::Unavailable)?;
    if response.status() != StatusCode::OK {
        return Ok(None);
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if content_type.split(';').next() != Some("image/png")
        || response.content_length().is_some_and(|len| len > 262_144)
    {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| IconError::Unavailable)? {
        if bytes.len() + chunk.len() > 262_144 {
            return Ok(None);
        }
        bytes.extend_from_slice(&chunk);
    }
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") || bytes.len() < 33 {
        return Ok(None);
    }
    Ok(Some(bytes))
}

#[cfg(test)]
pub(crate) mod tests;
