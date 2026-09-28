//! Local Redis/Valkey only; no provider/network API calls.
use deadpool_redis::redis::cmd;
use domain::news::NewsCache;
use zora_infrastructure::{news::cache::RedisNewsCache, redis_pool};

#[tokio::test]
#[ignore = "Set TEST_REDIS_URL to an isolated local Redis/Valkey instance"]
async fn news_cache_ttls_budget_and_concurrent_reservations() {
    let pool =
        redis_pool(&std::env::var("TEST_REDIS_URL").expect("isolated TEST_REDIS_URL")).unwrap();
    let cache = RedisNewsCache::new(pool.clone());
    let mut conn = pool.get().await.unwrap();
    let key = format!("test:{}", uuid::Uuid::new_v4());
    assert!(cache.get(&key, false).await.unwrap().is_none());
    cache.put(&key, &[]).await.unwrap();
    assert_eq!(cache.get(&key, false).await.unwrap().unwrap().len(), 0);
    for (prefix, ttl) in [("", 300), ("stale:", 3600)] {
        let actual: i64 = cmd("TTL")
            .arg(format!("news:v1:{prefix}{key}"))
            .query_async(&mut conn)
            .await
            .unwrap();
        assert!((ttl - 2..=ttl).contains(&actual));
    }
    // Only our feature's fixed budget keys are touched; use an isolated instance.
    let keys = [
        "news:v1:newsdata:budget",
        "news:v1:newsdata:window",
        "news:v1:newsdata:cooldown:crypto",
        "news:v1:newsdata:cooldown:market",
    ];
    cmd("DEL")
        .arg(&keys)
        .query_async::<usize>(&mut conn)
        .await
        .unwrap();
    let (a, b) = tokio::join!(
        cache.reserve_credit("crypto"),
        cache.reserve_credit("crypto")
    );
    assert_ne!(a.unwrap(), b.unwrap());
    assert!(cache.reserve_credit("market").await.unwrap());
    let count: i64 = cmd("HGET")
        .arg(keys[0])
        .arg("count")
        .query_async(&mut conn)
        .await
        .unwrap();
    assert_eq!(count, 2);
    cmd("DEL")
        .arg(&keys[2..])
        .query_async::<usize>(&mut conn)
        .await
        .unwrap();
    cmd("HSET")
        .arg(keys[0])
        .arg("count")
        .arg(179)
        .query_async::<usize>(&mut conn)
        .await
        .unwrap();
    assert!(cache.reserve_credit("crypto").await.unwrap());
    assert!(!cache.reserve_credit("market").await.unwrap());
    let count: i64 = cmd("HGET")
        .arg(keys[0])
        .arg("count")
        .query_async(&mut conn)
        .await
        .unwrap();
    assert_eq!(count, 180);
    // Reset daily count while retaining a full rolling window, including day boundaries.
    cmd("HSET")
        .arg(keys[0])
        .arg("day")
        .arg(-1)
        .query_async::<usize>(&mut conn)
        .await
        .unwrap();
    let now: Vec<String> = cmd("TIME").query_async(&mut conn).await.unwrap();
    for i in 0..30 {
        cmd("ZADD")
            .arg(keys[1])
            .arg(&now[0])
            .arg(format!("test:{i}"))
            .query_async::<usize>(&mut conn)
            .await
            .unwrap();
    }
    assert!(!cache.reserve_credit("market").await.unwrap());
    cmd("DEL")
        .arg(keys[1])
        .query_async::<usize>(&mut conn)
        .await
        .unwrap();
    assert!(cache.reserve_credit("market").await.unwrap());
    let count: i64 = cmd("HGET")
        .arg(keys[0])
        .arg("count")
        .query_async(&mut conn)
        .await
        .unwrap();
    assert_eq!(count, 1);
    cmd("DEL")
        .arg(&keys)
        .arg(format!("news:v1:{key}"))
        .arg(format!("news:v1:stale:{key}"))
        .query_async::<usize>(&mut conn)
        .await
        .unwrap();
}
