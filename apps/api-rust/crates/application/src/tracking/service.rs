use super::*;
use chrono::{SecondsFormat, Utc};
use domain::wallet::WalletRepository;
use futures_util::{StreamExt, TryStreamExt, stream};
use std::{str::FromStr, sync::Arc, time::Duration};

pub struct WalletTrackingService {
    wallets: Arc<dyn WalletRepository>,
    chain: Arc<dyn OnChainProvider>,
    prices: Arc<dyn PriceProvider>,
    repository: Arc<dyn TrackingRepository>,
    cache: Arc<dyn TrackingCache>,
}
impl WalletTrackingService {
    pub fn new(
        wallets: Arc<dyn WalletRepository>,
        chain: Arc<dyn OnChainProvider>,
        prices: Arc<dyn PriceProvider>,
        repository: Arc<dyn TrackingRepository>,
        cache: Arc<dyn TrackingCache>,
    ) -> Self {
        Self {
            wallets,
            chain,
            prices,
            repository,
            cache,
        }
    }
    pub async fn assets(&self, user: &str, id: &str, force: bool) -> TrackingResult<TrackedWallet> {
        // Authorization always precedes Redis, rate limits and paid HTTP calls.
        let wallet = self
            .wallets
            .find_by_id(id, user)
            .await
            .map_err(|_| TrackingError::StorageUnavailable)?
            .ok_or(TrackingError::NotFound)?;
        chain_info(wallet.chain)?;
        let key = format!("tracking:v1:{user}:{id}");
        if !force && let Some(cached) = self.cache.get(&key).await? {
            return Ok(cached);
        }
        let lease = self
            .cache
            .acquire(&key)
            .await?
            .ok_or(TrackingError::RateLimited)?;
        let result = tokio::time::timeout(Duration::from_secs(60), async {
            if !force && let Some(cached) = self.cache.get(&key).await? {
                return Ok(cached);
            }
            if force && !self.cache.allow_sync(&key).await? {
                return Err(TrackingError::RateLimited);
            }
            self.cache.invalidate(&key).await?;
            let info = chain_info(wallet.chain)?;
            let (native, tokens) = tokio::try_join!(
                self.chain.native_balance(wallet.chain, &wallet.address),
                self.chain.token_balances(wallet.chain, &wallet.address)
            )?;
            let mut complete = tokens.complete && tokens.tokens.len() <= MAX_TOKENS;
            let mut observed = vec!["native".to_owned()];
            let mut nonzero = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for token in tokens.tokens.into_iter().take(MAX_TOKENS) {
                let token = TokenBalance {
                    contract: token.contract.to_ascii_lowercase(),
                    raw: token.raw,
                };
                if !seen.insert(token.contract.clone()) {
                    continue;
                }
                if hex_amount(&token.raw, 0)? != "0" {
                    nonzero.push(token);
                } else {
                    observed.push(token.contract);
                }
            }
            let metadata: Vec<_> = stream::iter(nonzero.into_iter().map(|token| async move {
                let meta = self
                    .chain
                    .token_metadata(wallet.chain, &token.contract)
                    .await?;
                Ok::<_, TrackingError>((token, meta))
            }))
            .buffered(8)
            .try_collect()
            .await?;
            complete &= metadata.iter().all(|(_, meta)| meta.is_some());
            let contracts: Vec<_> = metadata
                .iter()
                .filter(|(_, meta)| meta.is_some())
                .map(|(token, _)| token.contract.clone())
                .collect();
            let (native_price, prices) = tokio::join!(
                self.prices.native_price(wallet.chain),
                self.prices.token_prices(wallet.chain, &contracts)
            );
            // Keep balances when quotes are unavailable; completeness prohibits
            // treating unknown prices as a complete portfolio valuation.
            let native_price = native_price.unwrap_or_else(|_| {
                tracing::warn!("Native price unavailable");
                None
            });
            let prices = prices.unwrap_or_else(|_| {
                tracing::warn!("Token prices unavailable");
                HashMap::new()
            });
            let mut assets = Vec::new();
            let native_amount = hex_amount(&native, 18)?;
            if native_amount != "0" {
                assets.push(asset(
                    TokenMetadata {
                        symbol: info.symbol.into(),
                        name: info.name.into(),
                        decimals: 18,
                    },
                    None,
                    native_amount,
                    native_price,
                )?);
            }
            for (token, meta) in metadata {
                if let Some(meta) = meta {
                    observed.push(token.contract.clone());
                    let amount = hex_amount(&token.raw, meta.decimals)?;
                    let price = prices.get(&token.contract).cloned();
                    assets.push(asset(meta, Some(token.contract), amount, price)?);
                }
            }
            let total_usd = value_and_sort(&mut assets)?;
            let result = TrackedWallet {
                wallet_id: wallet.id.clone(),
                address: wallet.address.to_ascii_lowercase(),
                chain: wallet.chain.as_str().into(),
                total_usd,
                complete,
                valuation_complete: complete
                    && assets.iter().all(|asset| asset.value_usd.is_some()),
                assets,
                updated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            };
            self.repository
                .save(user, &result, &observed, complete)
                .await?;
            self.cache.put(&key, &result).await?;
            Ok(result)
        })
        .await
        .unwrap_or(Err(TrackingError::ProviderUnavailable));
        // Compare-and-delete avoids releasing another worker's lease after expiration.
        let _ = self.cache.release(&key, &lease).await;
        result
    }
}
fn asset(
    meta: TokenMetadata,
    contract_address: Option<String>,
    amount: String,
    price: Option<String>,
) -> TrackingResult<TrackedAsset> {
    let price_usd = price
        .map(|p| price_decimal(&p).map(|v| v.normalized().to_plain_string()))
        .transpose()?;
    Ok(TrackedAsset {
        symbol: meta.symbol,
        name: meta.name,
        is_native: contract_address.is_none(),
        contract_address,
        amount,
        price_usd,
        value_usd: None,
        decimals: meta.decimals,
    })
}
fn value_and_sort(assets: &mut [TrackedAsset]) -> TrackingResult<Option<String>> {
    let mut total = BigDecimal::from(0);
    let mut priced = assets.is_empty();
    let mut values = HashMap::new();
    for asset in assets.iter_mut() {
        if let Some(price) = &asset.price_usd {
            let amount = BigDecimal::from_str(&asset.amount)
                .map_err(|_| TrackingError::ProviderUnavailable)?;
            let value = amount * price_decimal(price)?;
            total += &value;
            priced = true;
            asset.value_usd = Some(value.normalized().to_plain_string());
            values.insert(asset.key().to_owned(), value);
        }
    }
    assets.sort_by(|a, b| {
        values
            .get(b.key())
            .cmp(&values.get(a.key()))
            .then_with(|| a.key().cmp(b.key()))
    });
    Ok(priced.then(|| total.normalized().to_plain_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decimal_ordering_and_partial_total() {
        let mut assets = Vec::new();
        for (key, amount, price) in [
            ("a", "1", Some("2")),
            ("b", "1", Some("10")),
            ("c", "100", None),
        ] {
            assets.push(
                asset(
                    TokenMetadata {
                        symbol: "X".into(),
                        name: "X".into(),
                        decimals: 0,
                    },
                    Some(key.into()),
                    amount.into(),
                    price.map(Into::into),
                )
                .unwrap(),
            );
        }
        assert_eq!(value_and_sort(&mut assets).unwrap().as_deref(), Some("12"));
        assert_eq!(
            assets.iter().map(TrackedAsset::key).collect::<Vec<_>>(),
            vec!["b", "a", "c"]
        );
        assert_eq!(value_and_sort(&mut []).unwrap().as_deref(), Some("0"));
    }
}
