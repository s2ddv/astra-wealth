//! Icon identities and asynchronous ports. No HTTP, Redis or serialization dependencies.
use garde::Validate;
use std::{future::Future, pin::Pin};
use thiserror::Error;

pub type IconFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, IconError>> + Send + 'a>>;
#[derive(Debug, Error)]
pub enum IconError {
    #[error("Invalid asset reference")]
    InvalidReference,
    #[error("Icon dependency unavailable")]
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq, Validate)]
pub struct CryptoRef {
    #[garde(custom(valid_id))]
    pub coingecko_id: Option<String>,
    #[garde(custom(valid_crypto_symbol))]
    pub symbol: String,
    #[garde(skip)]
    pub token: Option<TokenRef>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenRef {
    pub chain: TokenChain,
    /// The infrastructure adapter normalizes EVM addresses to EIP-55.
    pub contract: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenChain {
    Ethereum,
    Polygon,
    Arbitrum,
    Base,
    Smartchain,
    Solana,
}
impl TokenChain {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ethereum => "ethereum",
            Self::Polygon => "polygon",
            Self::Arbitrum => "arbitrum",
            Self::Base => "base",
            Self::Smartchain => "smartchain",
            Self::Solana => "solana",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Validate)]
pub struct StockRef {
    #[garde(custom(valid_symbol))]
    pub ticker: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetRef {
    Crypto(CryptoRef),
    Stock(StockRef),
}

fn valid_crypto_symbol(value: &str, context: &()) -> garde::Result {
    if value.is_empty() {
        Ok(())
    } else {
        valid_symbol(value, context)
    }
}
fn valid_symbol(value: &str, _: &()) -> garde::Result {
    if !value.is_empty()
        && value.len() <= 24
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b".-".contains(&c))
        && value.bytes().any(|c| c.is_ascii_alphanumeric())
    {
        Ok(())
    } else {
        Err(garde::Error::new("Invalid symbol"))
    }
}
fn valid_id(value: &Option<String>, _: &()) -> garde::Result {
    if value.as_ref().is_none_or(|v| {
        !v.is_empty()
            && v.len() <= 100
            && v.bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
            && !v.starts_with('-')
    }) {
        Ok(())
    } else {
        Err(garde::Error::new("Invalid CoinGecko id"))
    }
}
impl AssetRef {
    pub fn validate(&self) -> Result<(), IconError> {
        match self {
            Self::Stock(value) => value.validate().map_err(|_| IconError::InvalidReference),
            Self::Crypto(value) => {
                value.validate().map_err(|_| IconError::InvalidReference)?;
                if value.symbol.is_empty() && value.coingecko_id.is_none() {
                    return Err(IconError::InvalidReference);
                }
                if let Some(token) = &value.token {
                    let valid =
                        match token.chain {
                            TokenChain::Solana => (32..=44).contains(&token.contract.len())
                                && token.contract.bytes().all(|c| {
                                    b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
                                        .contains(&c)
                                }),
                            _ => {
                                token.contract.len() == 42
                                    && token.contract.starts_with("0x")
                                    && token.contract[2..].bytes().all(|c| c.is_ascii_hexdigit())
                            }
                        };
                    if !valid {
                        return Err(IconError::InvalidReference);
                    }
                }
                Ok(())
            }
        }
    }
    pub fn symbol(&self) -> String {
        match self {
            Self::Crypto(v) => {
                if v.symbol.is_empty() {
                    v.coingecko_id.as_deref().unwrap_or("?")
                } else {
                    &v.symbol
                }
            }
            Self::Stock(v) => &v.ticker,
        }
        .to_ascii_uppercase()
    }
    pub fn cache_key(&self) -> String {
        match self {
            Self::Stock(v) => format!("icon:stock:{}", v.ticker.to_ascii_uppercase()),
            Self::Crypto(v) => {
                let identity = if let Some(id) = &v.coingecko_id {
                    format!("id:{id}")
                } else if let Some(t) = &v.token {
                    format!(
                        "token:{}:{}",
                        t.chain.as_str(),
                        if t.chain == TokenChain::Solana {
                            t.contract.clone()
                        } else {
                            t.contract.to_ascii_lowercase()
                        }
                    )
                } else {
                    format!("symbol:{}", v.symbol.to_ascii_uppercase())
                };
                format!("icon:crypto:{identity}")
            }
        }
    }
    /// Symbols aren't globally unique; never trust a previously cached symbol lookup.
    pub fn is_symbol_only(&self) -> bool {
        matches!(self, Self::Crypto(v) if v.coingecko_id.is_none() && v.token.is_none())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IconUrl(String);
impl IconUrl {
    /// Only provider-owned HTTPS hosts or downloaded PNG bytes may leave the service.
    pub fn new(value: String) -> Option<Self> {
        let remote = [
            "https://coin-images.coingecko.com/",
            "https://assets.coingecko.com/",
            "https://financialmodelingprep.com/image-stock/",
        ]
        .iter()
        .any(|prefix| value.starts_with(prefix));
        let png = value
            .strip_prefix("data:image/png;base64,")
            .is_some_and(|data| {
                !data.is_empty()
                    && data.len() <= 350_000
                    && data
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"+/=".contains(&c))
            });
        (value.len() <= 350_024 && !value.chars().any(char::is_control) && (remote || png))
            .then_some(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IconResponse {
    pub url: Option<IconUrl>,
    /// The frontend renders initials and hashes this normalized symbol for its color.
    pub symbol: String,
}

pub trait AssetIconProvider: Send + Sync {
    fn resolve_icon<'a>(&'a self, asset_ref: &'a AssetRef) -> IconFuture<'a, Option<IconUrl>>;
}
/// Separate opt-in port: never composed into the crypto market pipeline.
pub trait StockIconProvider: AssetIconProvider {}
pub trait AssetCache: Send + Sync {
    fn get<'a>(&'a self, key: &'a str) -> IconFuture<'a, Option<String>>;
    fn set<'a>(&'a self, key: &'a str, value: &'a str, ttl_seconds: u64) -> IconFuture<'a, ()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn id_only_lookup_is_valid_but_an_empty_reference_is_not() {
        let reference = CryptoRef {
            coingecko_id: Some("bitcoin".into()),
            symbol: String::new(),
            token: None,
        };
        assert!(AssetRef::Crypto(reference.clone()).validate().is_ok());
        assert!(
            AssetRef::Crypto(CryptoRef {
                coingecko_id: None,
                ..reference
            })
            .validate()
            .is_err()
        );
    }
    #[test]
    fn invalid_identifiers_and_contracts_are_rejected() {
        for id in ["../etc", "bitcoin?x=y", "BTC", ""] {
            let reference = CryptoRef {
                coingecko_id: Some(id.into()),
                symbol: "BTC".into(),
                token: None,
            };
            assert!(AssetRef::Crypto(reference).validate().is_err());
        }
        for contract in [
            "../logo.png",
            "0x12",
            "0xzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
        ] {
            let reference = CryptoRef {
                coingecko_id: None,
                symbol: "USDT".into(),
                token: Some(TokenRef {
                    chain: TokenChain::Ethereum,
                    contract: contract.into(),
                }),
            };
            assert!(AssetRef::Crypto(reference).validate().is_err());
        }
    }
    #[test]
    fn urls_allow_only_approved_sources_and_never_raw_github() {
        for value in [
            "javascript:alert(1)",
            "http://coin-images.coingecko.com/a.png",
            "https://coin-images.coingecko.com.evil/a.png",
            "https://raw.githubusercontent.com/trustwallet/assets/logo.png",
            "data:image/svg+xml,<svg />",
        ] {
            assert!(IconUrl::new(value.into()).is_none());
        }
        assert!(IconUrl::new("https://coin-images.coingecko.com/coins/a.png".into()).is_some());
    }
}
