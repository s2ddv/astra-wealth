use application::tracking::*;
use deadpool_redis::{Pool, redis::cmd};
use std::time::Duration;

pub struct RedisTrackingCache {
    pool: Pool,
}
impl RedisTrackingCache {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
    async fn command<T: deadpool_redis::redis::FromRedisValue>(
        &self,
        command: &mut deadpool_redis::redis::Cmd,
    ) -> TrackingResult<T> {
        tokio::time::timeout(Duration::from_secs(1), async {
            let mut conn = self
                .pool
                .get()
                .await
                .map_err(|_| TrackingError::StorageUnavailable)?;
            command
                .query_async(&mut conn)
                .await
                .map_err(|_| TrackingError::StorageUnavailable)
        })
        .await
        .map_err(|_| TrackingError::StorageUnavailable)?
    }
}
impl TrackingCache for RedisTrackingCache {
    fn get<'a>(&'a self, key: &'a str) -> TrackingFuture<'a, Option<TrackedWallet>> {
        Box::pin(async move {
            let value: Option<String> = self.command(cmd("GET").arg(key)).await?;
            Ok(value.and_then(|v| serde_json::from_str(&v).ok()))
        })
    }
    fn put<'a>(&'a self, key: &'a str, wallet: &'a TrackedWallet) -> TrackingFuture<'a, ()> {
        Box::pin(async move {
            let value =
                serde_json::to_string(wallet).map_err(|_| TrackingError::StorageUnavailable)?;
            self.command(cmd("SETEX").arg(key).arg(WALLET_TTL).arg(value))
                .await
        })
    }
    fn invalidate<'a>(&'a self, key: &'a str) -> TrackingFuture<'a, ()> {
        Box::pin(async move { self.command::<i64>(cmd("DEL").arg(key)).await.map(|_| ()) })
    }
    fn acquire<'a>(&'a self, key: &'a str) -> TrackingFuture<'a, Option<String>> {
        Box::pin(async move {
            let token = uuid::Uuid::new_v4().to_string();
            let result: Option<String> = self
                .command(
                    cmd("SET")
                        .arg(format!("{key}:lock"))
                        .arg(&token)
                        .arg("NX")
                        .arg("EX")
                        .arg(LEASE_TTL),
                )
                .await?;
            Ok(result.map(|_| token))
        })
    }
    fn release<'a>(&'a self, key: &'a str, token: &'a str) -> TrackingFuture<'a, ()> {
        Box::pin(async move {
            self.command::<i64>(cmd("EVAL")
                .arg("if redis.call('GET',KEYS[1]) == ARGV[1] then return redis.call('DEL',KEYS[1]) else return 0 end")
                .arg(1).arg(format!("{key}:lock")).arg(token)).await.map(|_| ())
        })
    }
    fn allow_sync<'a>(&'a self, key: &'a str) -> TrackingFuture<'a, bool> {
        Box::pin(async move {
            let result: Option<String> = self
                .command(
                    cmd("SET")
                        .arg(format!("{key}:sync"))
                        .arg("1")
                        .arg("NX")
                        .arg("EX")
                        .arg(SYNC_INTERVAL),
                )
                .await?;
            Ok(result.is_some())
        })
    }
}
