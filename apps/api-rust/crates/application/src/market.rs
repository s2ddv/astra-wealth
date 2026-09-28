use crate::asset_icon::AssetIconService;
use domain::{
    asset_icon::{AssetRef, CryptoRef, IconError, IconResponse},
    market::{CryptoMarketProvider, MarketCoin, MarketQuery},
};
use futures_util::{StreamExt, TryStreamExt, stream};
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
        self.market_page(MarketQuery::default()).await
    }
    pub async fn market_page(
        &self,
        query: MarketQuery,
    ) -> Result<Vec<(MarketCoin, IconResponse)>, IconError> {
        query.validate_query()?;
        self.resolve_all(self.provider.market_page(query).await?)
            .await
    }
    pub async fn trending(&self) -> Result<Vec<(MarketCoin, IconResponse)>, IconError> {
        self.resolve_all(self.provider.trending().await?).await
    }
    async fn resolve_all(
        &self,
        coins: Vec<MarketCoin>,
    ) -> Result<Vec<(MarketCoin, IconResponse)>, IconError> {
        stream::iter(coins.into_iter().map(|coin| async move {
            let icon = self.resolve(&coin).await?;
            Ok((coin, icon))
        }))
        .buffered(16)
        .try_collect()
        .await
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
