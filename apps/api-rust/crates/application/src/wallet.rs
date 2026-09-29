//! Wallet use cases. External balance providers are deliberately absent.
use domain::wallet::{NewWallet, NicknameUpdate, Wallet, WalletError, WalletRepository};
use std::sync::Arc;

#[derive(Clone)]
pub struct WalletService {
    repository: Arc<dyn WalletRepository>,
}
impl WalletService {
    pub fn new(repository: Arc<dyn WalletRepository>) -> Self {
        Self { repository }
    }
    pub async fn create(&self, user_id: &str, wallet: NewWallet) -> Result<Wallet, WalletError> {
        Ok(self.repository.create(user_id, wallet.validated()?).await?)
    }
    pub async fn list(&self, user_id: &str) -> Result<Vec<Wallet>, WalletError> {
        Ok(self.repository.find_by_user_id(user_id).await?)
    }
    pub async fn find_by_id(&self, id: &str, user_id: &str) -> Result<Option<Wallet>, WalletError> {
        Ok(self.repository.find_by_id(id, user_id).await?)
    }
    pub async fn update_nickname(
        &self,
        id: &str,
        user_id: &str,
        nickname: &str,
    ) -> Result<Option<Wallet>, WalletError> {
        let input = NicknameUpdate::validated(nickname)?;
        if self
            .repository
            .update_nickname(id, user_id, &input.nickname)
            .await?
            == 0
        {
            return Ok(None);
        }
        // Keep the ownership predicate on the response read too. Concurrent
        // deletion returns None rather than leaking/recreating another wallet.
        Ok(self.repository.find_by_id(id, user_id).await?)
    }
    pub async fn remove(&self, id: &str, user_id: &str) -> Result<bool, WalletError> {
        Ok(self.repository.delete(id, user_id).await? > 0)
    }
}
// WatchlistService will use its own repository port and the same authenticated
// local user id; do not couple it to wallet ownership or on-chain providers.
