//! Explicit, opt-in real-provider test. Never points to the project database.
use application::tracking::{OnChainProvider, PriceProvider, TrackingError};
use astra_infrastructure::{
    asset_icon::{RedisAssetCache, asset_http_client},
    postgres_pool, redis_pool,
    tracking::{AlchemyProvider, CoinGeckoPriceProvider, TrackingConfig, tracking_service},
};
use domain::wallet::Chain;
use std::sync::Arc;

#[tokio::test]
#[ignore = "Requires real CoinGecko access and isolated TEST_REDIS_URL"]
async fn real_coingecko_returns_eth_and_usdc_prices() {
    let redis = redis_pool(&std::env::var("TEST_REDIS_URL").unwrap()).unwrap();
    let provider = CoinGeckoPriceProvider::new(
        asset_http_client().unwrap(),
        Arc::new(RedisAssetCache::new(redis)),
        std::env::var("COINGECKO_API_KEY")
            .ok()
            .filter(|key| !key.is_empty()),
    );
    let native = provider
        .native_price(Chain::Ethereum)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(native, "0");
    let contract = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".to_owned();
    let prices = provider
        .token_prices(Chain::Ethereum, std::slice::from_ref(&contract))
        .await
        .unwrap();
    assert_ne!(prices.get(&contract).expect("USDC quote"), "0");
    println!("Real CoinGecko: ETH and USDC quotes verified");
}

#[tokio::test]
#[ignore = "Requires ALCHEMY_API_KEY and isolated TEST_DATABASE_URL / TEST_REDIS_URL"]
async fn real_alchemy_sync_persists_without_creating_contributions() {
    let db = std::env::var("TEST_DATABASE_URL").unwrap();
    assert!(
        db.contains("127.0.0.1:5432/astra_test"),
        "Only the disposable database is allowed"
    );
    let pool = postgres_pool(&db).unwrap();
    let redis = redis_pool(&std::env::var("TEST_REDIS_URL").unwrap()).unwrap();
    let provider = AlchemyProvider::new(
        asset_http_client().unwrap(),
        Arc::new(RedisAssetCache::new(redis.clone())),
        Some(std::env::var("ALCHEMY_API_KEY").expect("configure key without logging it")),
    );
    let meta = provider
        .token_metadata(
            Chain::Ethereum,
            "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(meta.symbol, "USDC");
    assert_eq!(meta.decimals, 6);
    let owner = uuid::Uuid::new_v4().to_string();
    let wallet = uuid::Uuid::new_v4().to_string();
    sqlx::query(r#"INSERT INTO users(id,email,"authId","updatedAt") VALUES($1,$2,$1,now())"#)
        .bind(&owner)
        .bind(format!("{owner}@test.invalid"))
        .execute(&pool)
        .await
        .unwrap();
    // Public checksum-example EOA; never a private user wallet.
    sqlx::query(r#"INSERT INTO wallets(id,address,chain,"userId","updatedAt") VALUES($1,'0xde709f2102306220921060314715629080e2fb77','ETHEREUM',$2,now())"#).bind(&wallet).bind(&owner).execute(&pool).await.unwrap();
    let service = tracking_service(pool.clone(), redis, TrackingConfig::from_env()).unwrap();
    let tracked = service.assets(&owner, &wallet, false).await.unwrap();
    let cached = service.assets(&owner, &wallet, false).await.unwrap();
    assert_eq!(tracked.updated_at, cached.updated_at);
    let persisted:(bool,bool,bool)=sqlx::query_as(r#"SELECT "lastSyncedAt" IS NOT NULL,"lastSyncComplete","lastValuationComplete" FROM wallets WHERE id=$1"#).bind(&wallet).fetch_one(&pool).await.unwrap();
    assert!(persisted.0);
    assert_eq!(persisted.1, tracked.complete);
    assert_eq!(persisted.2, tracked.valuation_complete);
    let count:i64=sqlx::query_scalar(r#"SELECT count(*) FROM "Contribution" c JOIN "FinancialAccount" a ON a.id=c."accountId" WHERE a."userId"=$1"#).bind(&owner).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 0);
    assert!(matches!(
        service.assets("other", &wallet, false).await,
        Err(TrackingError::NotFound)
    ));
    println!(
        "Real Alchemy: USDC metadata verified; sync persisted; {} assets; complete={}; valuationComplete={}; cached read verified",
        tracked.assets.len(),
        tracked.complete,
        tracked.valuation_complete
    );
    sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
}
