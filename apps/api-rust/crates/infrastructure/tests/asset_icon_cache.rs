use astra_infrastructure::{asset_icon::RedisAssetCache, redis_pool};
use domain::asset_icon::AssetCache;

#[tokio::test]
#[ignore = "Requires TEST_REDIS_URL pointing to an isolated Redis/Valkey instance"]
async fn resolved_icon_roundtrips_with_24_hour_ttl() {
    let pool = redis_pool(&std::env::var("TEST_REDIS_URL").expect("set TEST_REDIS_URL")).unwrap();
    let cache = RedisAssetCache::new(pool.clone());
    let key = format!("icon:crypto:id:test-{}", uuid::Uuid::new_v4());
    let value = "https://coin-images.coingecko.com/test.png";
    assert!(cache.get(&key).await.unwrap().is_none());
    cache
        .set(&key, value, application::asset_icon::ICON_TTL_SECONDS)
        .await
        .unwrap();
    assert_eq!(cache.get(&key).await.unwrap().as_deref(), Some(value));
    let mut connection = pool.get().await.unwrap();
    let ttl: i64 = deadpool_redis::redis::cmd("TTL")
        .arg(&key)
        .query_async(&mut connection)
        .await
        .unwrap();
    assert!((86395..=86400).contains(&ttl));
    let _: usize = deadpool_redis::redis::cmd("DEL")
        .arg(&key)
        .query_async(&mut connection)
        .await
        .unwrap();
}
