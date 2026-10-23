//! Historical client-supplied USD totals. Never mixed with BRL valuations.
use bigdecimal::{BigDecimal, RoundingMode};
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin, str::FromStr, sync::Arc};
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("Invalid totalUsd")]
    InvalidTotal,
    #[error("Invalid limit")]
    InvalidLimit,
    #[error("Snapshot storage unavailable")]
    Unavailable,
}
pub type SnapshotFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, SnapshotError>> + Send + 'a>>;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsdSnapshot {
    pub id: String,
    pub total_usd: String,
    pub user_id: String,
    pub created_at: String,
}
pub trait UsdSnapshotRepository: Send + Sync {
    fn list<'a>(&'a self, owner: &'a str, limit: i64) -> SnapshotFuture<'a, Vec<UsdSnapshot>>;
    fn create<'a>(&'a self, owner: &'a str, total: String) -> SnapshotFuture<'a, UsdSnapshot>;
}
#[derive(Clone)]
pub struct UsdSnapshotService {
    repository: Arc<dyn UsdSnapshotRepository>,
}
impl UsdSnapshotService {
    pub fn new(repository: Arc<dyn UsdSnapshotRepository>) -> Self {
        Self { repository }
    }
    pub async fn list(&self, owner: &str, limit: i64) -> Result<Vec<UsdSnapshot>, SnapshotError> {
        if !(1..=100).contains(&limit) {
            return Err(SnapshotError::InvalidLimit);
        }
        self.repository.list(owner, limit).await
    }
    pub async fn create(&self, owner: &str, total: &str) -> Result<UsdSnapshot, SnapshotError> {
        if total.len() > 128 {
            return Err(SnapshotError::InvalidTotal);
        }
        let value = BigDecimal::from_str(total).map_err(|_| SnapshotError::InvalidTotal)?;
        let (_, scale) = value.as_bigint_and_exponent();
        if !(-100..=100).contains(&scale) || value < 0 {
            return Err(SnapshotError::InvalidTotal);
        }
        let rounded = value.with_scale_round(2, RoundingMode::HalfUp);
        if rounded >= 10_000_000_000_000_000u64 {
            return Err(SnapshotError::InvalidTotal);
        }
        self.repository
            .create(owner, rounded.to_plain_string())
            .await
    }
}
