//! Concrete adapters; domain and application never depend on this crate.
use anyhow::{Context, ensure};
use deadpool_redis::{Config, Pool, Runtime};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::{str::FromStr, time::Duration};

pub fn postgres_pool(database_url: &str) -> anyhow::Result<PgPool> {
    let mut url = url::Url::parse(database_url).context("Invalid DATABASE_URL")?;
    ensure!(
        url.port_or_known_default().unwrap_or(5432) != 6543,
        "Supavisor port 6543 is unsupported; use direct Postgres on port 5432"
    );
    // Prisma's schema/pgbouncer options are not libpq connection options.
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| k != "schema" && k != "pgbouncer")
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    url.set_query(None);
    if !pairs.is_empty() {
        url.query_pairs_mut().extend_pairs(pairs);
    }
    let options = PgConnectOptions::from_str(url.as_str()).context("Invalid Postgres options")?;
    Ok(PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect_lazy_with(options))
}

pub fn redis_pool(redis_url: &str) -> anyhow::Result<Pool> {
    Config::from_url(redis_url)
        .create_pool(Some(Runtime::Tokio1))
        .context("Invalid REDIS_URL")
}

pub async fn health_checks(db: &PgPool, redis: &Pool) -> (bool, bool) {
    let database = async {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(db)
            .await
            .is_ok()
    };
    let cache = async {
        let Ok(mut conn) = redis.get().await else {
            return false;
        };
        deadpool_redis::redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await
            .is_ok_and(|value| value == "PONG")
    };
    let (db, cache) = tokio::join!(
        tokio::time::timeout(Duration::from_secs(5), database),
        tokio::time::timeout(Duration::from_secs(5), cache)
    );
    (db.unwrap_or(false), cache.unwrap_or(false))
}

/// Infrastructure handles stay out of the HTTP layer's dependencies.
pub struct HealthDependencies {
    db: PgPool,
    redis: Pool,
}
impl HealthDependencies {
    pub fn new(database_url: &str, redis_url: &str) -> anyhow::Result<Self> {
        Ok(Self {
            db: postgres_pool(database_url)?,
            redis: redis_pool(redis_url)?,
        })
    }
    pub async fn check(&self) -> (bool, bool) {
        health_checks(&self.db, &self.redis).await
    }
}

pub mod auth;
pub mod user;

pub mod wallet;

pub mod asset_icon;
pub mod market;

pub mod news;

pub mod accounts;

pub mod watchlist;
