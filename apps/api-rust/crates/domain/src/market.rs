use crate::asset_icon::{IconFuture, IconUrl};

#[derive(Clone, Debug)]
pub struct MarketCoin {
    pub id: String,
    pub symbol: String,
    pub name: String,
    pub price_usd: Option<f64>,
    pub image: Option<IconUrl>,
}
pub trait CryptoMarketProvider: Send + Sync {
    fn markets(&self) -> IconFuture<'_, Vec<MarketCoin>>;
    fn coin<'a>(&'a self, id: &'a str) -> IconFuture<'a, MarketCoin>;
}
