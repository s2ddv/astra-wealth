use domain::accounts::*;
use std::sync::Arc;

#[derive(Clone)]
pub struct AccountService {
    repository: Arc<dyn AccountRepository>,
}
impl AccountService {
    pub fn new(repository: Arc<dyn AccountRepository>) -> Self {
        Self { repository }
    }
    pub async fn list(&self, owner: &str) -> Result<Vec<Account>, AccountError> {
        self.repository.list(owner).await
    }
    pub async fn find(&self, owner: &str, id: &str) -> Result<AccountDetail, AccountError> {
        self.repository.find(owner, id).await
    }
    pub async fn create(
        &self,
        owner: &str,
        mut input: NewAccount,
    ) -> Result<Account, AccountError> {
        input.name = input.name.trim().into();
        input.institution_name = input
            .institution_name
            .map(|v| v.trim().to_owned())
            .filter(|v| !v.is_empty());
        input.validate()?;
        self.repository.create(owner, input).await
    }
    pub async fn remove(&self, owner: &str, id: &str) -> Result<(), AccountError> {
        self.repository.remove(owner, id).await
    }
    pub async fn save_holding(
        &self,
        owner: &str,
        account: &str,
        id: Option<&str>,
        input: NewHolding,
    ) -> Result<Holding, AccountError> {
        input.validate()?;
        self.repository
            .save_holding(owner, account, id, input)
            .await
    }
    pub async fn add_contribution(
        &self,
        owner: &str,
        account: &str,
        input: NewContribution,
    ) -> Result<Contribution, AccountError> {
        input.validate()?;
        self.repository
            .add_contribution(owner, account, input)
            .await
    }
    pub async fn remove_record(
        &self,
        owner: &str,
        account: &str,
        id: &str,
        holding: bool,
    ) -> Result<(), AccountError> {
        self.repository
            .remove_record(owner, account, id, holding)
            .await
    }
}
