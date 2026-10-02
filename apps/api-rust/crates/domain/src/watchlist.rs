//! Watchlist contract shared with the legacy API.
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistItem {
    pub id: String,
    pub coin_id: String,
    pub added_at: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Watchlist {
    pub id: String,
    pub name: String,
    pub user_id: String,
    pub items: Vec<WatchlistItem>,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug, thiserror::Error)]
pub enum WatchlistError {
    #[error("{message}")]
    Validation {
        field: &'static str,
        message: String,
    },
    #[error("Watchlist not found")]
    NotFound,
    #[error("Watchlist or item not found")]
    ItemNotFound,
    #[error("Watchlist name already exists")]
    NameConflict,
    #[error("Coin already in watchlist")]
    ItemConflict,
    #[error("Watchlist storage unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}
pub fn validated_text(
    value: &str,
    field: &'static str,
    max: usize,
) -> Result<String, WatchlistError> {
    let value = value.trim();
    let message = if value.is_empty() {
        Some("String must contain at least 1 character(s)".into())
    } else if value.encode_utf16().count() > max {
        Some(format!("String must contain at most {max} character(s)"))
    } else {
        None
    };
    if let Some(message) = message {
        return Err(WatchlistError::Validation { field, message });
    }
    Ok(value.into())
}
pub type WatchlistFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, WatchlistError>> + Send + 'a>>;
pub trait WatchlistRepository: Send + Sync {
    fn list<'a>(&'a self, owner: &'a str) -> WatchlistFuture<'a, Vec<Watchlist>>;
    fn create<'a>(&'a self, owner: &'a str, name: String) -> WatchlistFuture<'a, Watchlist>;
    fn remove<'a>(&'a self, owner: &'a str, id: &'a str) -> WatchlistFuture<'a, ()>;
    fn add_item<'a>(
        &'a self,
        owner: &'a str,
        id: &'a str,
        coin: String,
    ) -> WatchlistFuture<'a, WatchlistItem>;
    fn remove_item<'a>(
        &'a self,
        owner: &'a str,
        id: &'a str,
        coin: &'a str,
    ) -> WatchlistFuture<'a, ()>;
}
