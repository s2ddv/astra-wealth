//! Public, read-only Ethereum summary. Separate from authenticated wallet tracking.
use chrono::{SecondsFormat, Utc};
use domain::asset_icon::{AssetCache, IconError, IconFuture};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub trait NativeBalanceProvider: Send + Sync {
    fn balance<'a>(&'a self, address: &'a str) -> IconFuture<'a, String>;
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletBalance {
    pub address: String,
    pub native_balance: String,
    pub source: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletSummary {
    pub address: String,
    pub alchemy: Option<WalletBalance>,
    pub moralis: Option<WalletBalance>,
    pub fetched_at: String,
}
#[derive(Clone)]
pub struct PublicWalletService {
    alchemy: Arc<dyn NativeBalanceProvider>,
    moralis: Arc<dyn NativeBalanceProvider>,
    cache: Arc<dyn AssetCache>,
}
impl PublicWalletService {
    pub fn new(
        alchemy: Arc<dyn NativeBalanceProvider>,
        moralis: Arc<dyn NativeBalanceProvider>,
        cache: Arc<dyn AssetCache>,
    ) -> Self {
        Self {
            alchemy,
            moralis,
            cache,
        }
    }
    pub async fn summary(&self, address: &str) -> Result<WalletSummary, IconError> {
        if address.len() != 42
            || !address.starts_with("0x")
            || !address[2..].bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(IconError::InvalidReference);
        }
        let key = format!("wallet:{}", address.to_ascii_lowercase());
        if let Some(cached) = self.cache.get(&key).await? {
            return serde_json::from_str(&cached).map_err(|_| IconError::Unavailable);
        }
        let (alchemy, moralis) =
            tokio::join!(self.alchemy.balance(address), self.moralis.balance(address));
        let balance = |result: Result<String, IconError>, source: &str| {
            result.ok().map(|native_balance| WalletBalance {
                address: address.into(),
                native_balance,
                source: source.into(),
            })
        };
        let summary = WalletSummary {
            address: address.into(),
            alchemy: balance(alchemy, "alchemy"),
            moralis: balance(moralis, "moralis"),
            fetched_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        };
        let encoded = serde_json::to_string(&summary).map_err(|_| IconError::Unavailable)?;
        self.cache.set(&key, &encoded, 30).await?;
        Ok(summary)
    }
}

/// Six decimal ETH display, without the legacy intermediate IEEE-754 conversion.
pub fn display_wei(raw: &str, radix: u32) -> Result<String, IconError> {
    use bigdecimal::{BigDecimal, RoundingMode};
    use num_bigint::BigInt;
    let digits = if radix == 16 {
        raw.strip_prefix("0x").ok_or(IconError::Unavailable)?
    } else {
        raw
    };
    if digits.is_empty() || digits.len() > 78 || !digits.chars().all(|c| c.is_digit(radix)) {
        return Err(IconError::Unavailable);
    }
    let integer = BigInt::parse_bytes(digits.as_bytes(), radix).ok_or(IconError::Unavailable)?;
    if integer.bits() > 256 {
        return Err(IconError::Unavailable);
    }
    Ok(BigDecimal::new(integer, 18)
        .with_scale_round(6, RoundingMode::HalfUp)
        .to_plain_string())
}
