use domain::watchlist::*;
use std::sync::Arc;
#[derive(Clone)]
pub struct WatchlistService {
    repository: Arc<dyn WatchlistRepository>,
}
impl WatchlistService {
    pub fn new(repository: Arc<dyn WatchlistRepository>) -> Self {
        Self { repository }
    }
    pub async fn list(&self, owner: &str) -> Result<Vec<Watchlist>, WatchlistError> {
        self.repository.list(owner).await
    }
    pub async fn create(&self, owner: &str, name: &str) -> Result<Watchlist, WatchlistError> {
        self.repository
            .create(owner, validated_text(name, "name", 64)?)
            .await
    }
    pub async fn remove(&self, owner: &str, id: &str) -> Result<(), WatchlistError> {
        self.repository.remove(owner, id).await
    }
    pub async fn add_item(
        &self,
        owner: &str,
        id: &str,
        coin: &str,
    ) -> Result<WatchlistItem, WatchlistError> {
        self.repository
            .add_item(owner, id, validated_text(coin, "coinId", 128)?)
            .await
    }
    pub async fn remove_item(
        &self,
        owner: &str,
        id: &str,
        coin: &str,
    ) -> Result<(), WatchlistError> {
        self.repository.remove_item(owner, id, coin).await
    }
}
