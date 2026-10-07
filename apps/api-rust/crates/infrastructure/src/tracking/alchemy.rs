use super::bounded_json;
use application::tracking::*;
use domain::{
    asset_icon::AssetCache,
    wallet::{Chain, NewWallet},
};
use reqwest::Client;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};

pub struct AlchemyProvider {
    client: Client,
    cache: Arc<dyn AssetCache>,
    api_key: Option<String>,
    #[cfg(test)]
    pub(super) test_url: Option<String>,
}
impl AlchemyProvider {
    pub fn new(client: Client, cache: Arc<dyn AssetCache>, api_key: Option<String>) -> Self {
        Self {
            client,
            cache,
            api_key,
            #[cfg(test)]
            test_url: None,
        }
    }
    fn url(&self, chain: Chain) -> TrackingResult<String> {
        let network = chain_info(chain)?.network;
        #[cfg(test)]
        if let Some(url) = &self.test_url {
            return Ok(url.clone());
        }
        let key = self
            .api_key
            .as_ref()
            .filter(|key| {
                !key.is_empty()
                    && key
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            })
            .ok_or(TrackingError::ProviderUnavailable)?;
        Ok(format!("https://{network}.g.alchemy.com/v2/{key}"))
    }
    async fn rpc(&self, chain: Chain, method: &str, params: Value) -> TrackingResult<Value> {
        let url = self.url(chain)?;
        for attempt in 0..3 {
            let response = self
                .client
                .post(&url)
                .json(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}))
                .send()
                .await
                .map_err(|_| TrackingError::ProviderUnavailable)?;
            let status = response.status();
            if status.as_u16() == 429 || status.is_server_error() {
                if attempt < 2 {
                    tokio::time::sleep(Duration::from_millis(100 << attempt)).await;
                    continue;
                }
                return Err(if status.as_u16() == 429 {
                    TrackingError::RateLimited
                } else {
                    TrackingError::ProviderUnavailable
                });
            }
            if !status.is_success() {
                return Err(TrackingError::ProviderUnavailable);
            }
            let body = bounded_json(response).await?;
            if body["jsonrpc"] != "2.0" || body["id"] != 1 {
                return Err(TrackingError::ProviderUnavailable);
            }
            if !body["error"].is_null() {
                if body["error"]["code"].as_i64() == Some(429) {
                    if attempt < 2 {
                        tokio::time::sleep(Duration::from_millis(100 << attempt)).await;
                        continue;
                    }
                    return Err(TrackingError::RateLimited);
                }
                return Err(TrackingError::ProviderUnavailable);
            }
            return body
                .get("result")
                .cloned()
                .ok_or(TrackingError::ProviderUnavailable);
        }
        Err(TrackingError::ProviderUnavailable)
    }
}
impl OnChainProvider for AlchemyProvider {
    fn native_balance<'a>(&'a self, chain: Chain, address: &'a str) -> TrackingFuture<'a, String> {
        Box::pin(async move {
            let raw = self
                .rpc(chain, "eth_getBalance", json!([address, "latest"]))
                .await?;
            let raw = raw.as_str().ok_or(TrackingError::ProviderUnavailable)?;
            hex_amount(raw, 18)?;
            Ok(raw.into())
        })
    }
    fn token_balances<'a>(
        &'a self,
        chain: Chain,
        address: &'a str,
    ) -> TrackingFuture<'a, TokenBalances> {
        Box::pin(async move {
            // Bound work to the first 100 contracts. Preserve unobserved persisted
            // assets when a pageKey signals that the upstream result was truncated.
            let result = self
                .rpc(
                    chain,
                    "alchemy_getTokenBalances",
                    json!([address, "erc20", {"maxCount":MAX_TOKENS}]),
                )
                .await?;
            let rows = result["tokenBalances"]
                .as_array()
                .ok_or(TrackingError::ProviderUnavailable)?;
            let mut complete =
                result["pageKey"].as_str().is_none_or(str::is_empty) && rows.len() <= MAX_TOKENS;
            let mut tokens = Vec::new();
            for row in rows.iter().take(MAX_TOKENS) {
                if !row["error"].is_null() {
                    complete = false;
                    continue;
                }
                let contract = row["contractAddress"]
                    .as_str()
                    .ok_or(TrackingError::ProviderUnavailable)?;
                let validated = NewWallet {
                    address: contract.into(),
                    chain,
                    nickname: None,
                }
                .validated()
                .map_err(|_| TrackingError::ProviderUnavailable)?;
                let raw = row["tokenBalance"]
                    .as_str()
                    .ok_or(TrackingError::ProviderUnavailable)?;
                hex_amount(raw, 0)?;
                tokens.push(TokenBalance {
                    contract: validated.address,
                    raw: raw.into(),
                });
            }
            Ok(TokenBalances { tokens, complete })
        })
    }
    fn token_metadata<'a>(
        &'a self,
        chain: Chain,
        contract: &'a str,
    ) -> TrackingFuture<'a, Option<TokenMetadata>> {
        Box::pin(async move {
            let key = format!(
                "tracking:metadata:{}:{}",
                chain.as_str(),
                contract.to_ascii_lowercase()
            );
            if let Ok(Some(json)) = self.cache.get(&key).await
                && let Ok(metadata) = serde_json::from_str::<Option<TokenMetadata>>(&json)
            {
                return Ok(metadata);
            }
            let value = self
                .rpc(chain, "alchemy_getTokenMetadata", json!([contract]))
                .await?;
            let metadata = (|| {
                let symbol = value["symbol"].as_str()?.trim();
                let name = value["name"].as_str()?.trim();
                let decimals = u8::try_from(value["decimals"].as_u64()?).ok()?;
                if symbol.is_empty() || name.is_empty() || symbol.len() > 128 || name.len() > 512 {
                    return None;
                }
                Some(TokenMetadata {
                    symbol: symbol.into(),
                    name: name.into(),
                    decimals,
                })
            })();
            let json =
                serde_json::to_string(&metadata).map_err(|_| TrackingError::ProviderUnavailable)?;
            let _ = self.cache.set(&key, &json, METADATA_TTL).await;
            Ok(metadata)
        })
    }
}
