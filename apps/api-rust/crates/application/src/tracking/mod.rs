//! Read-only on-chain tracking ports and use case. No HTTP or SQL dependencies.
mod service;
use bigdecimal::BigDecimal;
use domain::wallet::Chain;
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};
pub use service::*;
use std::{collections::HashMap, future::Future, pin::Pin};
use thiserror::Error;

pub const MAX_TOKENS: usize = 100;
pub const WALLET_TTL: u64 = 30;
pub const PRICE_TTL: u64 = 10;
pub const METADATA_TTL: u64 = 86_400;
pub const SYNC_INTERVAL: u64 = 15;
pub const LEASE_TTL: u64 = 90;

#[derive(Debug, Error)]
pub enum TrackingError {
    #[error("Wallet not found")]
    NotFound,
    #[error("Chain not supported yet")]
    ChainNotSupported,
    #[error("Provider unavailable")]
    ProviderUnavailable,
    #[error("Rate limited")]
    RateLimited,
    #[error("Tracking storage unavailable")]
    StorageUnavailable,
}
pub type TrackingResult<T> = Result<T, TrackingError>;
pub type TrackingFuture<'a, T> = Pin<Box<dyn Future<Output = TrackingResult<T>> + Send + 'a>>;

pub struct ChainInfo {
    pub network: &'static str,
    pub platform: &'static str,
    pub coin_id: &'static str,
    pub symbol: &'static str,
    pub name: &'static str,
}
/// The only mapping of supported networks and CoinGecko identifiers.
pub fn chain_info(chain: Chain) -> TrackingResult<ChainInfo> {
    let (network, platform, coin_id, symbol, name) = match chain {
        Chain::Ethereum => ("eth-mainnet", "ethereum", "ethereum", "ETH", "Ether"),
        Chain::Polygon => (
            "polygon-mainnet",
            "polygon-pos",
            "polygon-ecosystem-token",
            "POL",
            "POL",
        ),
        Chain::Arbitrum => ("arb-mainnet", "arbitrum-one", "ethereum", "ETH", "Ether"),
        Chain::Base => ("base-mainnet", "base", "ethereum", "ETH", "Ether"),
        _ => return Err(TrackingError::ChainNotSupported),
    };
    Ok(ChainInfo {
        network,
        platform,
        coin_id,
        symbol,
        name,
    })
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenMetadata {
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
}
#[derive(Clone, Debug)]
pub struct TokenBalance {
    pub contract: String,
    pub raw: String,
}
#[derive(Default)]
pub struct TokenBalances {
    pub tokens: Vec<TokenBalance>,
    /// False when pagination/limits exclude tokens. Never delete unobserved assets then.
    pub complete: bool,
}
pub trait OnChainProvider: Send + Sync {
    fn native_balance<'a>(&'a self, chain: Chain, address: &'a str) -> TrackingFuture<'a, String>;
    fn token_balances<'a>(
        &'a self,
        chain: Chain,
        address: &'a str,
    ) -> TrackingFuture<'a, TokenBalances>;
    fn token_metadata<'a>(
        &'a self,
        chain: Chain,
        contract: &'a str,
    ) -> TrackingFuture<'a, Option<TokenMetadata>>;
}
pub trait PriceProvider: Send + Sync {
    fn native_price(&self, chain: Chain) -> TrackingFuture<'_, Option<String>>;
    fn token_prices<'a>(
        &'a self,
        chain: Chain,
        contracts: &'a [String],
    ) -> TrackingFuture<'a, HashMap<String, String>>;
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackedAsset {
    pub symbol: String,
    pub name: String,
    pub contract_address: Option<String>,
    pub amount: String,
    pub price_usd: Option<String>,
    pub value_usd: Option<String>,
    pub is_native: bool,
    pub decimals: u8,
}
impl TrackedAsset {
    pub fn key(&self) -> &str {
        self.contract_address.as_deref().unwrap_or("native")
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackedWallet {
    pub wallet_id: String,
    pub address: String,
    pub chain: String,
    pub total_usd: Option<String>,
    /// False means pagination or unavailable metadata excluded positions.
    #[serde(default)]
    pub complete: bool,
    /// Requires known USD prices for every position and complete balances.
    #[serde(default)]
    pub valuation_complete: bool,
    pub assets: Vec<TrackedAsset>,
    pub updated_at: String,
}
pub trait TrackingRepository: Send + Sync {
    /// Atomic upserts and pruning, with ownership checked again inside the transaction.
    fn save<'a>(
        &'a self,
        user: &'a str,
        wallet: &'a TrackedWallet,
        observed: &'a [String],
        complete: bool,
    ) -> TrackingFuture<'a, ()>;
}
pub trait TrackingCache: Send + Sync {
    fn get<'a>(&'a self, key: &'a str) -> TrackingFuture<'a, Option<TrackedWallet>>;
    fn put<'a>(&'a self, key: &'a str, wallet: &'a TrackedWallet) -> TrackingFuture<'a, ()>;
    fn invalidate<'a>(&'a self, key: &'a str) -> TrackingFuture<'a, ()>;
    /// Returns an opaque lease token; None means another worker owns this key.
    fn acquire<'a>(&'a self, key: &'a str) -> TrackingFuture<'a, Option<String>>;
    fn release<'a>(&'a self, key: &'a str, token: &'a str) -> TrackingFuture<'a, ()>;
    fn allow_sync<'a>(&'a self, key: &'a str) -> TrackingFuture<'a, bool>;
}

pub fn hex_amount(raw: &str, decimals: u8) -> TrackingResult<String> {
    let hex = raw
        .strip_prefix("0x")
        .ok_or(TrackingError::ProviderUnavailable)?;
    if hex.is_empty() || hex.len() > 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(TrackingError::ProviderUnavailable);
    }
    let integer =
        BigInt::parse_bytes(hex.as_bytes(), 16).ok_or(TrackingError::ProviderUnavailable)?;
    Ok(BigDecimal::new(integer, i64::from(decimals))
        .normalized()
        .to_plain_string())
}
pub fn price_decimal(value: &str) -> TrackingResult<BigDecimal> {
    use std::str::FromStr;
    if value.len() > 120 {
        return Err(TrackingError::ProviderUnavailable);
    }
    let number = BigDecimal::from_str(value).map_err(|_| TrackingError::ProviderUnavailable)?;
    let (_, scale) = number.as_bigint_and_exponent();
    if number < 0
        || !(-100..=100).contains(&scale)
        || number.digits() > 100
        || i128::from(number.digits()) - i128::from(scale) > 100
    {
        return Err(TrackingError::ProviderUnavailable);
    }
    Ok(number)
}

#[cfg(test)]
mod tests;
