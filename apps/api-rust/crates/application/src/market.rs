use crate::asset_icon::AssetIconService;
use domain::{
    asset_icon::{AssetRef, CryptoRef, IconError, IconResponse},
    market::{CryptoMarketProvider, MarketCoin},
};
use std::sync::Arc;

/// Phase 5 market ingestion: one upstream payload feeds both prices and icons.
#[derive(Clone)]
pub struct CryptoMarketService {
    provider: Arc<dyn CryptoMarketProvider>,
    icons: AssetIconService,
}
impl CryptoMarketService {
    pub fn new(provider: Arc<dyn CryptoMarketProvider>, icons: AssetIconService) -> Self {
        Self { provider, icons }
    }
    pub async fn markets(&self) -> Result<Vec<(MarketCoin, IconResponse)>, IconError> {
        let coins = self.provider.markets().await?;
        let mut resolved = Vec::with_capacity(coins.len());
        for coin in coins {
            let icon = self.resolve(&coin).await?;
            resolved.push((coin, icon));
        }
        Ok(resolved)
    }
    pub async fn coin(&self, id: &str) -> Result<(MarketCoin, IconResponse), IconError> {
        AssetRef::Crypto(CryptoRef {
            coingecko_id: Some(id.to_owned()),
            symbol: "coin".into(),
            token: None,
        })
        .validate()?;
        let coin = self.provider.coin(id).await?;
        let icon = self.resolve(&coin).await?;
        Ok((coin, icon))
    }
    async fn resolve(&self, coin: &MarketCoin) -> Result<IconResponse, IconError> {
        self.icons
            .resolve_icon(&AssetRef::Crypto(CryptoRef {
                coingecko_id: Some(coin.id.clone()),
                symbol: coin.symbol.clone(),
                token: None,
            }))
            .await
    }
}
