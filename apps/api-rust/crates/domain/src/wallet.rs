//! Wallet ownership and persistence ports. No on-chain operations.
use chrono::NaiveDateTime;
use std::{future::Future, pin::Pin, str::FromStr};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chain {
    Ethereum,
    Polygon,
    Arbitrum,
    Base,
    Solana,
    Bitcoin,
}
impl Chain {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ethereum => "ETHEREUM",
            Self::Polygon => "POLYGON",
            Self::Arbitrum => "ARBITRUM",
            Self::Base => "BASE",
            Self::Solana => "SOLANA",
            Self::Bitcoin => "BITCOIN",
        }
    }
}
impl FromStr for Chain {
    type Err = &'static str;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ETHEREUM" => Ok(Self::Ethereum),
            "POLYGON" => Ok(Self::Polygon),
            "ARBITRUM" => Ok(Self::Arbitrum),
            "BASE" => Ok(Self::Base),
            "SOLANA" => Ok(Self::Solana),
            "BITCOIN" => Ok(Self::Bitcoin),
            _ => Err("Invalid chain"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wallet {
    pub id: String,
    pub address: String,
    pub chain: Chain,
    pub nickname: Option<String>,
    pub user_id: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub last_synced_at: Option<NaiveDateTime>,
    /// Existing persisted assets are a read-only projection required by the
    /// Fastify wallet DTO. Asset mutation/synchronization is a later domain.
    pub assets: Vec<WalletAssetSummary>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalletAssetSummary {
    pub id: String,
    pub symbol: String,
    /// Exact decimal text, never floating point.
    pub amount: String,
    pub wallet_id: String,
    pub updated_at: NaiveDateTime,
}
#[derive(Clone, Debug, garde::Validate)]
pub struct NewWallet {
    #[garde(custom(address_rule(self.chain)))]
    pub address: String,
    #[garde(skip)]
    pub chain: Chain,
    #[garde(custom(optional_nickname_rule))]
    pub nickname: Option<String>,
}
impl NewWallet {
    pub fn validated(mut self) -> Result<Self, garde::Report> {
        self.nickname = self.nickname.map(|name| trim_nickname(&name).to_owned());
        garde::Validate::validate(&self)?;
        Ok(self)
    }
}
#[derive(Debug, garde::Validate)]
pub struct NicknameUpdate {
    #[garde(custom(nickname_rule))]
    pub nickname: String,
}
impl NicknameUpdate {
    pub fn validated(nickname: &str) -> Result<Self, garde::Report> {
        let input = Self {
            nickname: trim_nickname(nickname).to_owned(),
        };
        garde::Validate::validate(&input)?;
        Ok(input)
    }
}
/// ECMAScript trim and UTF-16 length match Zod's JavaScript string behavior.
fn trim_nickname(value: &str) -> &str {
    value.trim_matches(|c| matches!(c, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' |
        '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'))
}
fn nickname_rule(value: &str, _: &()) -> garde::Result {
    match value.encode_utf16().count() {
        0 => Err(garde::Error::new(
            "String must contain at least 1 character(s)",
        )),
        65.. => Err(garde::Error::new(
            "String must contain at most 64 character(s)",
        )),
        _ => Ok(()),
    }
}
fn optional_nickname_rule(value: &Option<String>, context: &()) -> garde::Result {
    value
        .as_ref()
        .map_or(Ok(()), |value| nickname_rule(value, context))
}
fn address_rule(chain: Chain) -> impl FnOnce(&str, &()) -> garde::Result {
    move |address, _| {
        let valid = match chain {
            Chain::Ethereum | Chain::Polygon | Chain::Arbitrum | Chain::Base => {
                evm_address(address)
            }
            Chain::Solana => {
                let mut bytes = [0u8; 32];
                (32..=44).contains(&address.len())
                    && bs58::decode(address)
                        .onto(&mut bytes)
                        .is_ok_and(|length| length == 32)
            }
            Chain::Bitcoin => {
                address.len() <= 90
                    && address
                        .parse::<bitcoin::Address<bitcoin::address::NetworkUnchecked>>()
                        .is_ok()
            }
        };
        if valid {
            Ok(())
        } else {
            Err(garde::Error::new(match chain {
                Chain::Solana => "Invalid Solana address",
                Chain::Bitcoin => "Invalid Bitcoin address",
                _ => "Invalid Ethereum address",
            }))
        }
    }
}
fn evm_address(address: &str) -> bool {
    use sha3::{Digest, Keccak256};
    let Some(hex) = address.strip_prefix("0x") else {
        return false;
    };
    if hex.len() != 40 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return false;
    }
    // Preserve conventional unchecksummed lower/upper case addresses; mixed
    // case must satisfy EIP-55 (Keccak-256, not standardized SHA3-256).
    if !hex.bytes().any(|byte| byte.is_ascii_lowercase())
        || !hex.bytes().any(|byte| byte.is_ascii_uppercase())
    {
        return true;
    }
    let hash = Keccak256::digest(hex.to_ascii_lowercase().as_bytes());
    hex.bytes().enumerate().all(|(index, byte)| {
        if byte.is_ascii_digit() {
            return true;
        }
        let nibble = if index % 2 == 0 {
            hash[index / 2] >> 4
        } else {
            hash[index / 2] & 15
        };
        byte.is_ascii_uppercase() == (nibble >= 8)
    })
}
#[derive(Debug, Error)]
pub enum WalletRepositoryError {
    #[error("Wallet is linked to a financial account")]
    LinkedAccount,
    #[error("Wallet already exists")]
    Conflict,
    #[error("Wallet repository unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}
pub type WalletResult<T> = Result<T, WalletRepositoryError>;
pub type WalletFuture<'a, T> = Pin<Box<dyn Future<Output = WalletResult<T>> + Send + 'a>>;

/// Ownership is mandatory in every method, including lookup and updates.
pub trait WalletRepository: Send + Sync {
    fn create<'a>(&'a self, user_id: &'a str, wallet: NewWallet) -> WalletFuture<'a, Wallet>;
    fn find_by_id<'a>(&'a self, id: &'a str, user_id: &'a str) -> WalletFuture<'a, Option<Wallet>>;
    fn find_by_user_id<'a>(&'a self, user_id: &'a str) -> WalletFuture<'a, Vec<Wallet>>;
    fn update_nickname<'a>(
        &'a self,
        id: &'a str,
        user_id: &'a str,
        nickname: &'a str,
    ) -> WalletFuture<'a, u64>;
    fn delete<'a>(&'a self, id: &'a str, user_id: &'a str) -> WalletFuture<'a, u64>;
}

#[derive(Debug, Error)]
pub enum WalletError {
    #[error(transparent)]
    Validation(#[from] garde::Report),
    #[error(transparent)]
    Repository(#[from] WalletRepositoryError),
}
