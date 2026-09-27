//! Business entities and ports. No database, HTTP or authentication SDK dependencies.
use chrono::NaiveDateTime;
use std::{future::Future, pin::Pin};
use thiserror::Error;

/// Prisma `User`: IDs are TEXT (including seeded non-UUID IDs), timestamps are
/// TIMESTAMP(3) without time zone, and `name` is nullable. `auth_id` is required.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub auth_id: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
#[derive(Clone, Debug)]
pub struct NewUser {
    pub id: Option<String>,
    pub email: String,
    pub name: Option<String>,
    pub auth_id: String,
}
#[derive(Clone, Debug, Default)]
pub struct UserUpdate {
    pub email: Option<String>,
    /// None leaves name unchanged; Some(None) clears it.
    pub name: Option<Option<String>>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthIdentity {
    pub auth_id: String,
    pub email: Option<String>,
}
#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("User already exists")]
    Conflict,
    #[error("User repository unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}
pub type RepositoryResult<T> = Result<T, RepositoryError>;
pub type RepositoryFuture<'a, T> = Pin<Box<dyn Future<Output = RepositoryResult<T>> + Send + 'a>>;

pub trait UserRepository: Send + Sync {
    fn find_by_id<'a>(&'a self, id: &'a str) -> RepositoryFuture<'a, Option<User>>;
    fn find_by_email<'a>(&'a self, email: &'a str) -> RepositoryFuture<'a, Option<User>>;
    fn create(&self, user: NewUser) -> RepositoryFuture<'_, User>;
    fn update<'a>(&'a self, id: &'a str, update: UserUpdate) -> RepositoryFuture<'a, Option<User>>;
    /// Atomic by authId, matching Fastify's Prisma upsert during authentication.
    fn upsert_identity(&self, identity: AuthIdentity) -> RepositoryFuture<'_, User>;
}

pub mod asset_icon;
pub mod market;
