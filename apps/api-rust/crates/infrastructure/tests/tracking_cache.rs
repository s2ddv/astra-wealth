use application::tracking::*;
use astra_infrastructure::{redis_pool, tracking::RedisTrackingCache};
use deadpool_redis::redis::cmd;
#[tokio::test]
#[ignore = "Requires isolated TEST_REDIS_URL"]
async fn ttl_cooldown_and_lease_ownership() {
    let pool = redis_pool(&std::env::var("TEST_REDIS_URL").unwrap()).unwrap();
    let cache = RedisTrackingCache::new(pool.clone());
    let key = format!("tracking:test:{}", uuid::Uuid::new_v4());
    let wallet = TrackedWallet {
        wallet_id: "w".into(),
        address: "a".into(),
        chain: "ETHEREUM".into(),
        total_usd: None,
        complete: true,
        valuation_complete: true,
        updated_at: "date".into(),
        assets: vec![],
    };
    cache.put(&key, &wallet).await.unwrap();
    assert_eq!(cache.get(&key).await.unwrap().unwrap().wallet_id, "w");
    let mut conn = pool.get().await.unwrap();
    let ttl: i64 = cmd("TTL").arg(&key).query_async(&mut conn).await.unwrap();
    assert!((29..=30).contains(&ttl));
    let token = cache.acquire(&key).await.unwrap().unwrap();
    assert!(cache.acquire(&key).await.unwrap().is_none());
    cache.release(&key, "different-owner").await.unwrap();
    assert!(cache.acquire(&key).await.unwrap().is_none());
    cache.release(&key, &token).await.unwrap();
    let next = cache.acquire(&key).await.unwrap().unwrap();
    assert_ne!(token, next);
    assert!(cache.allow_sync(&key).await.unwrap());
    assert!(!cache.allow_sync(&key).await.unwrap());
    let ttl: i64 = cmd("TTL")
        .arg(format!("{key}:sync"))
        .query_async(&mut conn)
        .await
        .unwrap();
    assert!((14..=15).contains(&ttl));
    cache.invalidate(&key).await.unwrap();
    assert!(cache.get(&key).await.unwrap().is_none());
    let _: () = cmd("SET")
        .arg(&key)
        .arg("corrupt")
        .query_async(&mut conn)
        .await
        .unwrap();
    assert!(cache.get(&key).await.unwrap().is_none());
    let _: i64 = cmd("DEL")
        .arg(&key)
        .arg(format!("{key}:sync"))
        .arg(format!("{key}:lock"))
        .query_async(&mut conn)
        .await
        .unwrap();
}
