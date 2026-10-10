//! USD/BRL valuation policy. Amounts and rates never pass through binary floats.
use chrono::{DateTime, Duration, FixedOffset, NaiveDate, Utc};
use domain::asset_icon::AssetCache;
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin, sync::Arc};

pub type FxFuture<'a> = Pin<Box<dyn Future<Output = Result<Option<FxQuote>, FxError>> + Send + 'a>>;
#[derive(Debug, thiserror::Error)]
pub enum FxError {
    #[error("Expected a date between 2000-01-01 and today in Brasilia")]
    InvalidDate,
    #[error("PTAX unavailable")]
    Unavailable,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FxQuote {
    pub rate: String,
    pub effective_date: String,
    /// BCB local timestamp, explicitly identified as America/Sao_Paulo in the contract.
    pub published_at: String,
}
pub trait FxProvider: Send + Sync {
    /// Latest closing sell quote in [date - 14 days, date]. Never a future quote.
    fn closing_on_or_before(&self, date: NaiveDate) -> FxFuture<'_>;
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FxRate {
    pub pair: String,
    pub source: String,
    pub requested_date: String,
    pub quote: Option<FxQuote>,
    /// available | previous_close | stale | missing | unavailable
    pub status: String,
    pub fetched_at: String,
}
#[derive(Clone)]
pub struct FxService {
    provider: Arc<dyn FxProvider>,
    cache: Arc<dyn AssetCache>,
}
pub fn brasilia_today(now: DateTime<Utc>) -> NaiveDate {
    now.with_timezone(&FixedOffset::west_opt(3 * 3600).expect("valid offset"))
        .date_naive()
}
impl FxService {
    pub fn new(provider: Arc<dyn FxProvider>, cache: Arc<dyn AssetCache>) -> Self {
        Self { provider, cache }
    }
    pub async fn rate(&self, date: NaiveDate, now: DateTime<Utc>) -> Result<FxRate, FxError> {
        let today = brasilia_today(now);
        if date > today || date < NaiveDate::from_ymd_opt(2000, 1, 1).unwrap() {
            return Err(FxError::InvalidDate);
        }
        let key = format!("fx:ptax:sell:v1:{date}");
        let cached = self
            .cache
            .get(&key)
            .await
            .ok()
            .flatten()
            .and_then(|v| serde_json::from_str::<FxRate>(&v).ok())
            .filter(|v| v.requested_date == date.to_string() && v.source == "BCB_PTAX_SELL_CLOSE")
            .filter(|v| v.quote.as_ref().is_none_or(|q| valid_quote(q, date)));
        let age = cached
            .as_ref()
            .and_then(|v| DateTime::parse_from_rfc3339(&v.fetched_at).ok())
            .map(|fetched| now.signed_duration_since(fetched));
        // Recent dates must be rechecked: a closing quote may still be published/corrected.
        let stable = date < today - Duration::days(7);
        let fresh_seconds = if cached.as_ref().is_some_and(|v| v.quote.is_none()) {
            60
        } else if stable {
            86400
        } else {
            300
        };
        if age.is_some_and(|age| age >= Duration::zero() && age < Duration::seconds(fresh_seconds))
        {
            return Ok(cached.unwrap());
        }
        let quote = match self.provider.closing_on_or_before(date).await {
            Ok(quote) if quote.as_ref().is_none_or(|q| valid_quote(q, date)) => quote,
            _ => {
                if age.is_some_and(|age| {
                    age >= Duration::zero() && age <= Duration::days(if stable { 90 } else { 1 })
                }) && let Some(mut cached) = cached.filter(|v| v.quote.is_some())
                {
                    cached.status = "stale".into();
                    return Ok(cached);
                }
                return Ok(FxRate {
                    pair: "USD/BRL".into(),
                    source: "BCB_PTAX_SELL_CLOSE".into(),
                    requested_date: date.to_string(),
                    quote: None,
                    status: "unavailable".into(),
                    fetched_at: now.to_rfc3339(),
                });
            }
        };
        let status = match &quote {
            None => "missing",
            Some(q) if q.effective_date != date.to_string() => "previous_close",
            Some(_) => "available",
        };
        let result = FxRate {
            pair: "USD/BRL".into(),
            source: "BCB_PTAX_SELL_CLOSE".into(),
            requested_date: date.to_string(),
            quote,
            status: status.into(),
            fetched_at: now.to_rfc3339(),
        };
        if let Ok(value) = serde_json::to_string(&result) {
            let _ = self
                .cache
                .set(&key, &value, if stable { 90 * 86400 } else { 86400 })
                .await;
        }
        Ok(result)
    }
}
pub fn valid_quote(quote: &FxQuote, requested: NaiveDate) -> bool {
    let Ok(date) = NaiveDate::parse_from_str(&quote.effective_date, "%Y-%m-%d") else {
        return false;
    };
    let Ok(published) =
        chrono::NaiveDateTime::parse_from_str(&quote.published_at, "%Y-%m-%d %H:%M:%S%.f")
    else {
        return false;
    };
    date <= requested
        && date >= requested - Duration::days(14)
        && published.date() == date
        && crate::tracking::price_decimal(&quote.rate).is_ok_and(|r| r > 0)
}

#[cfg(test)]
mod tests;
