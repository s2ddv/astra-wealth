//! User use cases, depending only on domain and the standard library.
use domain::{AuthIdentity, NewUser, RepositoryResult, User, UserRepository, UserUpdate};
use std::sync::Arc;

#[derive(Clone)]
pub struct UserService {
    repository: Arc<dyn UserRepository>,
}
impl UserService {
    pub fn new(repository: Arc<dyn UserRepository>) -> Self {
        Self { repository }
    }
    pub async fn find_by_id(&self, id: &str) -> RepositoryResult<Option<User>> {
        self.repository.find_by_id(id).await
    }
    pub async fn find_by_email(&self, email: &str) -> RepositoryResult<Option<User>> {
        self.repository.find_by_email(email).await
    }
    pub async fn create(&self, user: NewUser) -> RepositoryResult<User> {
        self.repository.create(user).await
    }
    pub async fn update(&self, id: &str, update: UserUpdate) -> RepositoryResult<Option<User>> {
        self.repository.update(id, update).await
    }
    pub async fn authenticate(&self, mut identity: AuthIdentity) -> RepositoryResult<User> {
        if identity.email.as_ref().is_none_or(String::is_empty) {
            identity.email = Some(format!("user-{}@supabase.local", identity.auth_id));
        }
        self.repository.upsert_identity(identity).await
    }
    // WalletService will receive the same repository port. The authenticated
    // user's local id, NOT the Supabase auth id, owns wallets.userId.
}

pub mod asset_icon;
pub mod market;
