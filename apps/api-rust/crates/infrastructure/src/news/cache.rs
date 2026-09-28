use deadpool_redis::{Pool, redis::cmd};
use domain::news::*;
use std::time::Duration;

#[derive(Clone)]
pub struct RedisNewsCache {
    pool: Pool,
}
impl RedisNewsCache {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
    async fn connection(&self) -> Result<deadpool_redis::Connection, NewsError> {
        tokio::time::timeout(Duration::from_secs(2), self.pool.get())
            .await
            .map_err(|_| NewsError::Unavailable)?
            .map_err(|_| NewsError::Unavailable)
    }
    pub async fn read(&self, key: &str) -> Result<Option<Vec<NewsArticle>>, NewsError> {
        let mut conn = self.connection().await?;
        let value: Option<String> = cmd("GET")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|_| NewsError::Unavailable)?;
        value
            .map(|s| serde_json::from_str(&s).map_err(|_| NewsError::Unavailable))
            .transpose()
    }
    pub async fn write(
        &self,
        key: &str,
        articles: &[NewsArticle],
        ttl: u64,
    ) -> Result<(), NewsError> {
        let value = serde_json::to_string(articles).map_err(|_| NewsError::Unavailable)?;
        let mut conn = self.connection().await?;
        cmd("SET")
            .arg(key)
            .arg(value)
            .arg("EX")
            .arg(ttl)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|_| NewsError::Unavailable)
    }
    /// Atomic across instances. Redis TIME supplies the UTC day and rolling window.
    /// Cooldown is reserved BEFORE I/O, including failed/ambiguous requests. Fail closed.
    pub async fn reserve_credit(&self, endpoint: &str) -> Result<bool, NewsError> {
        let mut conn = self.connection().await?;
        let allowed: i32 = cmd("EVAL")
            .arg(include_str!("reserve_credit.lua"))
            .arg(3)
            .arg("news:v1:newsdata:budget")
            .arg("news:v1:newsdata:window")
            .arg(format!("news:v1:newsdata:cooldown:{endpoint}"))
            .query_async(&mut conn)
            .await
            .map_err(|_| NewsError::Unavailable)?;
        Ok(allowed == 1)
    }
}
impl NewsCache for RedisNewsCache {
    fn get<'a>(&'a self, key: &'a str, stale: bool) -> NewsFuture<'a, Option<Vec<NewsArticle>>> {
        Box::pin(async move {
            self.read(&format!(
                "news:v1:{}{key}",
                if stale { "stale:" } else { "" }
            ))
            .await
        })
    }
    fn put<'a>(&'a self, key: &'a str, articles: &'a [NewsArticle]) -> NewsFuture<'a, ()> {
        Box::pin(async move {
            self.write(&format!("news:v1:stale:{key}"), articles, 3600)
                .await?;
            self.write(&format!("news:v1:{key}"), articles, 300).await
        })
    }
}
