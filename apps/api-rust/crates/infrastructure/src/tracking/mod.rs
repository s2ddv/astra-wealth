//! Alchemy, CoinGecko, Redis and PostgreSQL adapters for read-only tracking.
mod alchemy;
mod cache;
mod prices;
mod repository;
use crate::asset_icon::{RedisAssetCache, asset_http_client};
pub use alchemy::AlchemyProvider;
use application::tracking::*;
pub use cache::RedisTrackingCache;
pub use prices::CoinGeckoPriceProvider;
pub use repository::SqlxTrackingRepository;
use std::sync::Arc;

/// Secrets deliberately have no Debug implementation.
pub struct TrackingConfig {
    pub alchemy_api_key: Option<String>,
    pub coingecko_api_key: Option<String>,
}
impl TrackingConfig {
    pub fn from_env() -> Self {
        let setting = |key| std::env::var(key).ok().filter(|v| !v.trim().is_empty());
        Self {
            alchemy_api_key: setting("ALCHEMY_API_KEY"),
            coingecko_api_key: setting("COINGECKO_API_KEY"),
        }
    }
}
pub fn tracking_service(
    pool: sqlx::PgPool,
    redis: deadpool_redis::Pool,
    config: TrackingConfig,
) -> Result<WalletTrackingService, reqwest::Error> {
    let client = asset_http_client()?;
    let cache = Arc::new(RedisAssetCache::new(redis.clone()));
    Ok(WalletTrackingService::new(
        Arc::new(crate::wallet::SqlxWalletRepository::new(pool.clone())),
        Arc::new(AlchemyProvider::new(
            client.clone(),
            cache.clone(),
            config.alchemy_api_key,
        )),
        Arc::new(CoinGeckoPriceProvider::new(
            client,
            cache,
            config.coingecko_api_key,
        )),
        Arc::new(SqlxTrackingRepository::new(pool)),
        Arc::new(RedisTrackingCache::new(redis)),
    ))
}
async fn bounded_json(mut response: reqwest::Response) -> TrackingResult<serde_json::Value> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| TrackingError::ProviderUnavailable)?
    {
        if bytes.len() + chunk.len() > 1_000_000 {
            return Err(TrackingError::ProviderUnavailable);
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| TrackingError::ProviderUnavailable)
}
#[cfg(test)]
mod tests;
