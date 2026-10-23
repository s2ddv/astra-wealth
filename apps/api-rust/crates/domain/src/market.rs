use crate::asset_icon::{AssetRef, CryptoRef, IconError, IconFuture, IconUrl};
use garde::Validate;

#[derive(Clone, Debug, Default)]
pub struct MarketCoin {
    pub id: String,
    pub symbol: String,
    pub name: String,
    pub price_usd: Option<f64>,
    pub change_24h: Option<f64>,
    pub market_cap: Option<f64>,
    pub volume_24h: Option<f64>,
    pub rank: Option<u32>,
    pub updated_at: Option<String>,
    pub image: Option<IconUrl>,
}
#[derive(Clone, Debug, Default)]
pub enum MarketOrder {
    #[default]
    MarketCap,
    Change24h,
}
impl MarketOrder {
    pub fn provider_value(&self) -> &'static str {
        match self {
            Self::MarketCap => "market_cap_desc",
            Self::Change24h => "price_change_percentage_24h_desc",
        }
    }
}
#[derive(Clone, Debug, Validate)]
pub struct MarketQuery {
    #[garde(skip)]
    pub order: MarketOrder,
    #[garde(range(min = 1, max = 10000))]
    pub page: u32,
    #[garde(range(min = 1, max = 100))]
    pub per_page: u32,
    #[garde(length(max = 100))]
    pub ids: Vec<String>,
}
impl Default for MarketQuery {
    fn default() -> Self {
        Self {
            order: MarketOrder::default(),
            page: 1,
            per_page: 100,
            ids: Vec::new(),
        }
    }
}
impl MarketQuery {
    pub fn validate_query(&self) -> Result<(), IconError> {
        self.validate().map_err(|_| IconError::InvalidReference)?;
        for id in &self.ids {
            validate_coin_id(id)?;
        }
        Ok(())
    }
}
pub fn validate_coin_id(id: &str) -> Result<(), IconError> {
    AssetRef::Crypto(CryptoRef {
        coingecko_id: Some(id.into()),
        symbol: String::new(),
        token: None,
    })
    .validate()
}
pub trait CryptoMarketProvider: Send + Sync {
    fn markets(&self) -> IconFuture<'_, Vec<MarketCoin>> {
        self.market_page(MarketQuery::default())
    }
    fn market_page(&self, query: MarketQuery) -> IconFuture<'_, Vec<MarketCoin>>;
    fn trending(&self) -> IconFuture<'_, Vec<MarketCoin>>;
    fn coin<'a>(&'a self, id: &'a str) -> IconFuture<'a, MarketCoin>;
}
