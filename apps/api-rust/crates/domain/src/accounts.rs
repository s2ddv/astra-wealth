//! Financial accounts and manual records. Transfers never create contributions.
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin};

#[derive(Debug, thiserror::Error)]
pub enum AccountError {
    #[error("{0}")]
    Validation(&'static str),
    #[error("Account or record not found")]
    NotFound,
    #[error("Record already exists")]
    Conflict,
    #[error("Account storage unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}
pub type AccountFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, AccountError>> + Send + 'a>>;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewAccount {
    pub name: String,
    pub institution_name: Option<String>,
    pub kind: String,
    pub base_currency: String,
    pub wallet_id: Option<String>,
}
impl NewAccount {
    pub fn validate(&self) -> Result<(), AccountError> {
        text(&self.name, 120)?;
        if let Some(name) = &self.institution_name {
            text(name, 120)?;
        }
        currency(&self.base_currency)?;
        if !["MANUAL", "BANK", "BROKERAGE", "WALLET"].contains(&self.kind.as_str()) {
            return Err(AccountError::Validation("Unsupported account kind"));
        }
        if (self.kind == "WALLET") != self.wallet_id.is_some() {
            return Err(AccountError::Validation(
                "Only wallet accounts require walletId",
            ));
        }
        if let Some(id) = &self.wallet_id {
            text(id, 200)?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewHolding {
    pub asset_class: String,
    pub symbol: Option<String>,
    pub name: String,
    pub quantity: String,
    pub unit_price: Option<String>,
    pub current_value: String,
    pub due_date: Option<String>,
}
impl NewHolding {
    pub fn validate(&self) -> Result<(), AccountError> {
        text(&self.name, 200)?;
        if let Some(symbol) = &self.symbol {
            text(symbol, 40)?;
        }
        if ![
            "CRYPTO",
            "STOCK",
            "REAL_ESTATE_FUND",
            "ETF",
            "FIXED_INCOME",
            "MUTUAL_FUND",
            "PENSION",
            "CASH",
            "OTHER",
        ]
        .contains(&self.asset_class.as_str())
        {
            return Err(AccountError::Validation("Invalid assetClass"));
        }
        decimal(&self.quantity, 18, 10, false)?;
        decimal(&self.current_value, 18, 10, false)?;
        if let Some(price) = &self.unit_price {
            decimal(price, 18, 10, false)?;
        }
        if let Some(date) = &self.due_date {
            timestamp(date)?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewContribution {
    pub kind: String,
    pub amount: String,
    pub currency: String,
    pub fx_rate_to_brl: Option<String>,
    pub occurred_at: String,
    pub note: Option<String>,
}
impl NewContribution {
    pub fn validate(&self) -> Result<(), AccountError> {
        if !["DEPOSIT", "WITHDRAWAL"].contains(&self.kind.as_str()) {
            return Err(AccountError::Validation("Invalid contribution kind"));
        }
        decimal(&self.amount, 18, 10, true)?;
        currency(&self.currency)?;
        if self.currency == "USD" && self.fx_rate_to_brl.is_none() {
            return Err(AccountError::Validation(
                "USD requires historical fxRateToBrl",
            ));
        }
        if let Some(rate) = &self.fx_rate_to_brl {
            decimal(rate, 10, 8, true)?;
        }
        let time = timestamp(&self.occurred_at)?;
        if time > chrono::Utc::now() {
            return Err(AccountError::Validation(
                "occurredAt cannot be in the future",
            ));
        }
        if self
            .note
            .as_ref()
            .is_some_and(|note| note.chars().count() > 1000)
        {
            return Err(AccountError::Validation("Note exceeds 1000 characters"));
        }
        Ok(())
    }
}
pub fn timestamp(value: &str) -> Result<chrono::DateTime<chrono::Utc>, AccountError> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|v| v.with_timezone(&chrono::Utc))
        .map_err(|_| AccountError::Validation("Expected RFC3339 timestamp"))
}
fn text(value: &str, max: usize) -> Result<(), AccountError> {
    if value.trim().is_empty() || value.chars().count() > max {
        return Err(AccountError::Validation("Invalid text length"));
    }
    Ok(())
}
fn currency(value: &str) -> Result<(), AccountError> {
    if !["BRL", "USD"].contains(&value) {
        return Err(AccountError::Validation("Unsupported currency"));
    }
    Ok(())
}
pub fn decimal(
    value: &str,
    integer: usize,
    scale: usize,
    positive: bool,
) -> Result<(), AccountError> {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty()
        || whole.len() > integer
        || !whole.bytes().all(|c| c.is_ascii_digit())
        || fraction.len() > scale
        || !fraction.bytes().all(|c| c.is_ascii_digit())
        || (value.contains('.') && fraction.is_empty())
        || (positive && !value.bytes().any(|c| matches!(c, b'1'..=b'9')))
    {
        return Err(AccountError::Validation(
            "Invalid decimal precision or sign",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub institution_name: Option<String>,
    pub kind: String,
    pub base_currency: String,
    pub wallet_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Holding {
    pub id: String,
    pub account_id: String,
    pub asset_class: String,
    pub symbol: Option<String>,
    pub name: String,
    pub quantity: String,
    pub unit_price: Option<String>,
    pub current_value: String,
    pub due_date: Option<String>,
    pub origin: String,
    pub external_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contribution {
    pub id: String,
    pub account_id: String,
    pub kind: String,
    pub amount: String,
    pub currency: String,
    pub fx_rate_to_brl: Option<String>,
    pub occurred_at: String,
    pub note: Option<String>,
    pub origin: String,
    pub external_id: Option<String>,
    pub created_at: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDetail {
    #[serde(flatten)]
    pub account: Account,
    pub holdings: Vec<Holding>,
    pub contributions: Vec<Contribution>,
}
pub trait AccountRepository: Send + Sync {
    fn list<'a>(&'a self, owner: &'a str) -> AccountFuture<'a, Vec<Account>>;
    fn find<'a>(&'a self, owner: &'a str, id: &'a str) -> AccountFuture<'a, AccountDetail>;
    fn create<'a>(&'a self, owner: &'a str, input: NewAccount) -> AccountFuture<'a, Account>;
    fn remove<'a>(&'a self, owner: &'a str, id: &'a str) -> AccountFuture<'a, ()>;
    fn save_holding<'a>(
        &'a self,
        owner: &'a str,
        account: &'a str,
        id: Option<&'a str>,
        input: NewHolding,
    ) -> AccountFuture<'a, Holding>;
    fn add_contribution<'a>(
        &'a self,
        owner: &'a str,
        account: &'a str,
        input: NewContribution,
    ) -> AccountFuture<'a, Contribution>;
    fn remove_record<'a>(
        &'a self,
        owner: &'a str,
        account: &'a str,
        id: &'a str,
        holding: bool,
    ) -> AccountFuture<'a, ()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decimals_are_exact_and_bounded() {
        for value in [
            "1e3",
            "NaN",
            "-1",
            "1.",
            ".2",
            "0.00000000001",
            "1000000000000000000",
            "１",
        ] {
            assert!(decimal(value, 18, 10, false).is_err(), "{value}");
        }
        assert!(decimal("999999999999999999.1234567891", 18, 10, true).is_ok());
        assert!(decimal("0.0000000000", 18, 10, true).is_err());
        assert!(decimal("0.0000000001", 18, 10, true).is_ok());
    }
    #[test]
    fn historical_fx_and_dates_are_required() {
        let mut input = NewContribution {
            kind: "DEPOSIT".into(),
            amount: "1".into(),
            currency: "USD".into(),
            fx_rate_to_brl: None,
            occurred_at: "2020-01-01T00:00:00Z".into(),
            note: None,
        };
        assert!(input.validate().is_err());
        input.fx_rate_to_brl = Some("5.12345678".into());
        assert!(input.validate().is_ok());
        input.occurred_at = "2999-01-01T00:00:00Z".into();
        assert!(input.validate().is_err());
    }
    #[test]
    fn wallet_link_is_explicit_and_exchanges_are_deferred() {
        let mut input = NewAccount {
            name: "Savings".into(),
            institution_name: None,
            kind: "WALLET".into(),
            base_currency: "USD".into(),
            wallet_id: None,
        };
        assert!(input.validate().is_err());
        input.wallet_id = Some("owned-wallet".into());
        assert!(input.validate().is_ok());
        input.kind = "MANUAL".into();
        assert!(input.validate().is_err());
        input.kind = "EXCHANGE".into();
        assert!(input.validate().is_err());
    }
}
