//! Point-in-time consolidation. Transfers and balances are never contributions.
use crate::fx::{FxRate, FxService, brasilia_today};
use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, Utc};
use domain::wallet::Chain;
use futures_util::{StreamExt, stream};
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin, str::FromStr, sync::Arc};

#[derive(Debug, thiserror::Error)]
#[error("Portfolio storage unavailable")]
pub struct PortfolioError;
pub type PortfolioFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, PortfolioError>> + Send + 'a>>;
#[derive(Clone)]
pub enum PositionKind {
    Fiat {
        currency: String,
        value: String,
    },
    Crypto {
        chain: Chain,
        contract: Option<String>,
        amount: String,
    },
    EmptyWallet,
    Unsupported,
}
#[derive(Clone)]
pub struct PositionInput {
    pub id: String,
    pub account_id: Option<String>,
    pub wallet_id: Option<String>,
    pub name: String,
    pub updated_at: String,
    pub issue: Option<String>,
    pub kind: PositionKind,
}
#[derive(Clone)]
pub struct ContributionInput {
    pub id: String,
    pub amount: String,
    pub currency: String,
    pub withdrawal: bool,
    pub date: NaiveDate,
    pub recorded_rate: Option<String>,
}
pub struct PortfolioInput {
    pub observed_at: String,
    pub positions: Vec<PositionInput>,
    pub contributions: Vec<ContributionInput>,
}
pub trait PortfolioRepository: Send + Sync {
    /// One repeatable-read view, scoped to the authenticated local user ID.
    fn load<'a>(&'a self, owner: &'a str) -> PortfolioFuture<'a, PortfolioInput>;
    fn save<'a>(
        &'a self,
        owner: &'a str,
        snapshot: &'a PortfolioSnapshot,
    ) -> PortfolioFuture<'a, String>;
    fn history<'a>(&'a self, owner: &'a str) -> PortfolioFuture<'a, Vec<SavedSnapshot>>;
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrlQuote {
    pub price: String,
    pub fetched_at: String,
}
pub trait BrlPriceProvider: Send + Sync {
    fn price<'a>(
        &'a self,
        chain: Chain,
        contract: Option<&'a str>,
    ) -> PortfolioFuture<'a, Option<BrlQuote>>;
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValuedPosition {
    pub id: String,
    pub account_id: Option<String>,
    pub wallet_id: Option<String>,
    pub name: String,
    pub value_brl: Option<String>,
    pub source: String,
    pub input_value: Option<String>,
    pub currency: Option<String>,
    pub unit_price_brl: Option<String>,
    pub price_fetched_at: Option<String>,
    pub fx: Option<FxRate>,
    pub balance_updated_at: String,
    pub issue: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValuedContribution {
    pub id: String,
    pub value_brl: Option<String>,
    pub rate: Option<String>,
    pub source: String,
    pub occurred_on: String,
    pub fx: Option<FxRate>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioSnapshot {
    pub version: u32,
    pub observed_at: String,
    pub valued_at: String,
    /// Sum only of known values. Null when no position is valued; zero for an empty portfolio.
    pub known_value_brl: Option<String>,
    pub complete: bool,
    pub net_contributions_brl: Option<String>,
    pub contributions_complete: bool,
    /// Unavailable for any partial valuation or incomplete contribution history.
    pub result_brl: Option<String>,
    pub positions: Vec<ValuedPosition>,
    pub contributions: Vec<ValuedContribution>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSnapshot {
    pub id: String,
    pub snapshot: PortfolioSnapshot,
}
#[derive(Clone)]
pub struct PortfolioService {
    repository: Arc<dyn PortfolioRepository>,
    fx: FxService,
    prices: Arc<dyn BrlPriceProvider>,
}
impl PortfolioService {
    pub fn new(
        repository: Arc<dyn PortfolioRepository>,
        fx: FxService,
        prices: Arc<dyn BrlPriceProvider>,
    ) -> Self {
        Self {
            repository,
            fx,
            prices,
        }
    }
    pub async fn current(
        &self,
        owner: &str,
        now: DateTime<Utc>,
    ) -> Result<PortfolioSnapshot, PortfolioError> {
        let input = self.repository.load(owner).await?;
        self.consolidate(input, now).await
    }
    pub async fn capture(
        &self,
        owner: &str,
        now: DateTime<Utc>,
    ) -> Result<SavedSnapshot, PortfolioError> {
        let snapshot = self.current(owner, now).await?;
        let id = self.repository.save(owner, &snapshot).await?;
        Ok(SavedSnapshot { id, snapshot })
    }
    pub async fn history(&self, owner: &str) -> Result<Vec<SavedSnapshot>, PortfolioError> {
        self.repository.history(owner).await
    }
    async fn consolidate(
        &self,
        input: PortfolioInput,
        now: DateTime<Utc>,
    ) -> Result<PortfolioSnapshot, PortfolioError> {
        let needs_fx = input
            .positions
            .iter()
            .any(|p| matches!(&p.kind, PositionKind::Fiat { currency, .. } if currency == "USD"));
        let current_fx = if needs_fx {
            self.fx.rate(brasilia_today(now), now).await.ok()
        } else {
            None
        };
        let positions: Vec<_> = stream::iter(
            input
                .positions
                .into_iter()
                .map(|p| self.value_position(p, current_fx.clone())),
        )
        .buffered(6)
        .collect()
        .await;
        let mut contributions = Vec::new();
        // Memoize historical dates within this request even if Redis is unavailable.
        let mut rates = std::collections::HashMap::new();
        for c in input.contributions {
            let (rate, source, fx) = if c.currency == "BRL" {
                (Some("1".into()), "BRL", None)
            } else if c.currency == "USD" && c.recorded_rate.is_some() {
                (c.recorded_rate, "RECORDED", None)
            } else if c.currency == "USD" {
                if let std::collections::hash_map::Entry::Vacant(entry) = rates.entry(c.date) {
                    entry.insert(self.fx.rate(c.date, now).await.ok());
                }
                let fx = rates.get(&c.date).cloned().flatten();
                let rate = fx
                    .as_ref()
                    .filter(|f| f.status != "stale")
                    .and_then(|f| f.quote.as_ref())
                    .map(|q| q.rate.clone());
                (rate, "BCB_PTAX_SELL_CLOSE", fx)
            } else {
                (None, "UNSUPPORTED", None)
            };
            let value_brl = rate
                .as_ref()
                .and_then(|r| multiply(&c.amount, r))
                .map(|v| if c.withdrawal { plain(-v) } else { plain(v) });
            contributions.push(ValuedContribution {
                id: c.id,
                value_brl,
                rate,
                source: source.into(),
                occurred_on: c.date.to_string(),
                fx,
            });
        }
        let complete = positions
            .iter()
            .all(|p| p.value_brl.is_some() && p.issue.is_none());
        let contributions_complete = contributions.iter().all(|c| c.value_brl.is_some());
        let known_value_brl = sum(positions.iter().map(|p| p.value_brl.as_deref()));
        let net_contributions_brl = if contributions_complete {
            sum(contributions.iter().map(|c| c.value_brl.as_deref()))
        } else {
            None
        };
        let result_brl = if complete && contributions_complete {
            Some(plain(
                BigDecimal::from_str(known_value_brl.as_deref().ok_or(PortfolioError)?)
                    .map_err(|_| PortfolioError)?
                    - BigDecimal::from_str(net_contributions_brl.as_deref().ok_or(PortfolioError)?)
                        .map_err(|_| PortfolioError)?,
            ))
        } else {
            None
        };
        Ok(PortfolioSnapshot {
            version: 1,
            observed_at: input.observed_at,
            valued_at: now.to_rfc3339(),
            known_value_brl,
            complete,
            net_contributions_brl,
            contributions_complete,
            result_brl,
            positions,
            contributions,
        })
    }
    async fn value_position(&self, p: PositionInput, current_fx: Option<FxRate>) -> ValuedPosition {
        let mut v = ValuedPosition {
            id: p.id,
            account_id: p.account_id,
            wallet_id: p.wallet_id,
            name: p.name,
            value_brl: None,
            source: "UNAVAILABLE".into(),
            input_value: None,
            currency: None,
            unit_price_brl: None,
            price_fetched_at: None,
            fx: None,
            balance_updated_at: p.updated_at,
            issue: p.issue,
        };
        match p.kind {
            PositionKind::Fiat { currency, value } => {
                v.source = "RECORDED_HOLDING".into();
                v.currency = Some(currency.clone());
                v.input_value = Some(value.clone());
                if currency == "BRL" {
                    v.value_brl = multiply(&value, "1").map(plain);
                } else if currency == "USD" {
                    v.fx = current_fx;
                    v.value_brl =
                        v.fx.as_ref()
                            .and_then(|fx| fx.quote.as_ref())
                            .and_then(|q| multiply(&value, &q.rate))
                            .map(plain);
                    if v.fx.as_ref().is_some_and(|f| f.status == "stale") {
                        v.issue = Some("STALE_FX".into());
                    }
                }
            }
            PositionKind::Crypto {
                chain,
                contract,
                amount,
            } => {
                v.source = "COINGECKO_BRL".into();
                v.input_value = Some(amount.clone());
                let quote = tokio::time::timeout(
                    std::time::Duration::from_secs(12),
                    self.prices.price(chain, contract.as_deref()),
                )
                .await
                .ok()
                .and_then(Result::ok)
                .flatten();
                if let Some(quote) = quote {
                    v.value_brl = multiply(&amount, &quote.price).map(plain);
                    v.unit_price_brl = Some(quote.price);
                    v.price_fetched_at = Some(quote.fetched_at);
                }
            }
            PositionKind::EmptyWallet => {
                v.source = "SYNCED_BALANCE".into();
                if v.issue.is_none() {
                    v.value_brl = Some("0".into());
                }
            }
            PositionKind::Unsupported => {
                v.issue.get_or_insert_with(|| "UNSUPPORTED_ASSET".into());
            }
        }
        if v.value_brl.is_none() && v.issue.is_none() {
            v.issue = Some("MISSING_QUOTE".into());
        }
        v
    }
}
fn multiply(a: &str, b: &str) -> Option<BigDecimal> {
    // Wallet balances can have up to 255 decimal places; keep every digit.
    if a.len() > 800 {
        return None;
    }
    let amount = BigDecimal::from_str(a).ok()?;
    let (_, scale) = amount.as_bigint_and_exponent();
    if amount < 0 || !(-400..=400).contains(&scale) {
        return None;
    }
    let rate = crate::tracking::price_decimal(b).ok()?;
    if rate <= 0 {
        return None;
    }
    Some(amount * rate)
}
fn plain(value: BigDecimal) -> String {
    value.normalized().to_plain_string()
}
fn sum<'a>(values: impl Iterator<Item = Option<&'a str>>) -> Option<String> {
    let mut total = BigDecimal::from(0);
    let mut count = 0;
    let mut priced = 0;
    for value in values {
        count += 1;
        if let Some(value) = value {
            total += BigDecimal::from_str(value).ok()?;
            priced += 1;
        }
    }
    (count == 0 || priced > 0).then(|| plain(total))
}
#[cfg(test)]
mod tests;
