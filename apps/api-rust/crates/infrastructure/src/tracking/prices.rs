use super::bounded_json;
use application::tracking::*;
use domain::{asset_icon::AssetCache, wallet::Chain};
use reqwest::Client;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};

/// Shares the Phase 5 HTTP client/cache adapter and Demo API authentication.
/// Separate simple-price payloads avoid converting through market DTO f64 values.
pub struct CoinGeckoPriceProvider {
    client: Client,
    cache: Arc<dyn AssetCache>,
    api_key: Option<String>,
    pub(super) base: String,
}
impl CoinGeckoPriceProvider {
    pub fn new(client: Client, cache: Arc<dyn AssetCache>, api_key: Option<String>) -> Self {
        Self {
            client,
            cache,
            api_key,
            base: "https://api.coingecko.com/api/v3".into(),
        }
    }
    async fn payload(&self, path: &str, query: &[(&str, &str)]) -> TrackingResult<Value> {
        let mut url = url::Url::parse(&format!("{}{path}", self.base))
            .map_err(|_| TrackingError::ProviderUnavailable)?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let mut request = self.client.get(url);
        if let Some(key) = &self.api_key {
            request = request.header("x-cg-demo-api-key", key);
        }
        let response = request
            .send()
            .await
            .map_err(|_| TrackingError::ProviderUnavailable)?;
        if response.status().as_u16() == 429 {
            return Err(TrackingError::RateLimited);
        }
        if !response.status().is_success() {
            return Err(TrackingError::ProviderUnavailable);
        }
        let value = bounded_json(response).await?;
        if !value.is_object() || value.get("error").is_some() || value.get("status").is_some() {
            return Err(TrackingError::ProviderUnavailable);
        }
        Ok(value)
    }
    async fn cached_price(&self, key: &str) -> Option<Option<String>> {
        let cached = self.cache.get(key).await.ok()??;
        serde_json::from_str(&cached).ok()
    }
    async fn cache_price(&self, key: &str, price: &Option<String>) {
        if let Ok(value) = serde_json::to_string(price) {
            let _ = self.cache.set(key, &value, PRICE_TTL).await;
        }
    }
}
fn usd(value: &Value) -> TrackingResult<Option<String>> {
    if value.is_null() || value["usd"].is_null() {
        return Ok(None);
    }
    let number = value["usd"]
        .as_number()
        .ok_or(TrackingError::ProviderUnavailable)?;
    Ok(Some(
        price_decimal(&number.to_string())?
            .normalized()
            .to_plain_string(),
    ))
}
impl PriceProvider for CoinGeckoPriceProvider {
    fn native_price(&self, chain: Chain) -> TrackingFuture<'_, Option<String>> {
        Box::pin(async move {
            let id = chain_info(chain)?.coin_id;
            let key = format!("tracking:price:native:{id}");
            if let Some(price) = self.cached_price(&key).await {
                return Ok(price);
            }
            let value = self
                .payload(
                    "/simple/price",
                    &[("ids", id), ("vs_currencies", "usd"), ("precision", "full")],
                )
                .await?;
            let price = usd(&value[id])?;
            self.cache_price(&key, &price).await;
            Ok(price)
        })
    }
    fn token_prices<'a>(
        &'a self,
        chain: Chain,
        contracts: &'a [String],
    ) -> TrackingFuture<'a, HashMap<String, String>> {
        Box::pin(async move {
            let platform = chain_info(chain)?.platform;
            let mut prices = HashMap::new();
            let mut missing = Vec::new();
            for contract in contracts.iter().take(MAX_TOKENS) {
                let contract = contract.to_ascii_lowercase();
                let key = format!("tracking:price:{platform}:{contract}");
                match self.cached_price(&key).await {
                    Some(Some(price)) => {
                        prices.insert(contract, price);
                    }
                    Some(None) => {}
                    None => missing.push(contract),
                }
            }
            // Demo accounts may only allow one contract per request. Use a
            // bounded sequential fill; outer service deadline caps total work.
            for contract in missing {
                let value = self
                    .payload(
                        &format!("/simple/token_price/{platform}"),
                        &[
                            ("contract_addresses", &contract),
                            ("vs_currencies", "usd"),
                            ("precision", "full"),
                        ],
                    )
                    .await?;
                let price = usd(&value[&contract])?;
                self.cache_price(&format!("tracking:price:{platform}:{contract}"), &price)
                    .await;
                if let Some(price) = price {
                    prices.insert(contract, price);
                }
            }
            Ok(prices)
        })
    }
}

// Separate BRL contract/cache: legacy USD tracking remains unchanged.
impl application::portfolio::BrlPriceProvider for CoinGeckoPriceProvider {
    fn price<'a>(
        &'a self,
        chain: Chain,
        contract: Option<&'a str>,
    ) -> application::portfolio::PortfolioFuture<'a, Option<application::portfolio::BrlQuote>> {
        Box::pin(async move {
            use application::portfolio::{BrlQuote, PortfolioError};
            let info = chain_info(chain).map_err(|_| PortfolioError)?;
            let contract = contract.map(str::to_ascii_lowercase);
            if contract.as_ref().is_some_and(|c| {
                c.len() != 42
                    || !c.starts_with("0x")
                    || !c[2..].bytes().all(|b| b.is_ascii_hexdigit())
            }) {
                return Err(PortfolioError);
            }
            let key = format!(
                "portfolio:coingecko:brl:v1:{}:{}",
                info.platform,
                contract.as_deref().unwrap_or("native")
            );
            if let Some(cached) = self.cache.get(&key).await.ok().flatten()
                && let Ok(quote) = serde_json::from_str::<BrlQuote>(&cached)
                && chrono::DateTime::parse_from_rfc3339(&quote.fetched_at).is_ok_and(|t| {
                    let age = chrono::Utc::now().signed_duration_since(t).num_seconds();
                    (0..60).contains(&age)
                })
                && price_decimal(&quote.price).is_ok_and(|p| p > 0)
            {
                return Ok(Some(quote));
            }
            let (path, parameter, identity) = if let Some(c) = contract.as_deref() {
                (
                    format!("/simple/token_price/{}", info.platform),
                    "contract_addresses",
                    c,
                )
            } else {
                ("/simple/price".into(), "ids", info.coin_id)
            };
            let body = self
                .payload(
                    &path,
                    &[
                        (parameter, identity),
                        ("vs_currencies", "brl"),
                        ("precision", "full"),
                    ],
                )
                .await
                .map_err(|_| PortfolioError)?;
            let Some(number) = body[identity]["brl"].as_number() else {
                return Ok(None);
            };
            let price = price_decimal(&number.to_string()).map_err(|_| PortfolioError)?;
            if price <= 0 {
                return Ok(None);
            }
            let quote = BrlQuote {
                price: price.normalized().to_plain_string(),
                fetched_at: chrono::Utc::now().to_rfc3339(),
            };
            if let Ok(payload) = serde_json::to_string(&quote) {
                let _ = self.cache.set(&key, &payload, 60).await;
            }
            Ok(Some(quote))
        })
    }
}
