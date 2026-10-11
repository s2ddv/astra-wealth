use application::{
    fx::{FxFuture, FxProvider, FxQuote, FxService},
    portfolio::*,
};
use astra_infrastructure::{
    asset_icon::RedisAssetCache, portfolio::SqlxPortfolioRepository, postgres_pool, redis_pool,
};
use chrono::{NaiveDate, Utc};
use domain::wallet::Chain;
use std::sync::Arc;
struct Fx;
impl FxProvider for Fx {
    fn closing_on_or_before(&self, date: NaiveDate) -> FxFuture<'_> {
        Box::pin(async move {
            Ok(Some(FxQuote {
                rate: "5.1234".into(),
                effective_date: date.to_string(),
                published_at: format!("{date} 13:00:00"),
            }))
        })
    }
}
struct Prices;
impl BrlPriceProvider for Prices {
    fn price<'a>(&'a self, _: Chain, _: Option<&'a str>) -> PortfolioFuture<'a, Option<BrlQuote>> {
        Box::pin(async {
            Ok(Some(BrlQuote {
                price: "10000.123456789".into(),
                fetched_at: Utc::now().to_rfc3339(),
            }))
        })
    }
}
#[tokio::test]
#[ignore = "Requires disposable TEST_DATABASE_URL and TEST_REDIS_URL"]
async fn real_storage_consolidates_once_preserves_rates_and_isolates_snapshots() {
    let url = std::env::var("TEST_DATABASE_URL").unwrap();
    let parsed = url::Url::parse(&url).unwrap();
    assert_eq!(parsed.host_str(), Some("127.0.0.1"));
    assert!(["/astra_ptax_test", "/astra_operations_test"].contains(&parsed.path()));
    let pool = postgres_pool(&url).unwrap();
    let repo = Arc::new(SqlxPortfolioRepository::new(pool.clone()));
    let redis = redis_pool(&std::env::var("TEST_REDIS_URL").unwrap()).unwrap();
    let service = PortfolioService::new(
        repo.clone(),
        FxService::new(Arc::new(Fx), Arc::new(RedisAssetCache::new(redis))),
        Arc::new(Prices),
    );
    let owner = uuid::Uuid::new_v4().to_string();
    let other = uuid::Uuid::new_v4().to_string();
    let account = uuid::Uuid::new_v4().to_string();
    let linked = uuid::Uuid::new_v4().to_string();
    let wallet = uuid::Uuid::new_v4().to_string();
    for id in [&owner, &other] {
        sqlx::query(r#"INSERT INTO users(id,email,"authId","updatedAt") VALUES($1,$2,$1,now())"#)
            .bind(id)
            .bind(format!("{id}@test.invalid"))
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query(r#"INSERT INTO wallets(id,address,chain,"userId","updatedAt","lastSyncedAt","lastSyncComplete") VALUES($1,'0xde709f2102306220921060314715629080e2fb77','ETHEREUM',$2,now(),now(),true)"#).bind(&wallet).bind(&owner).execute(&pool).await.unwrap();
    sqlx::query(r#"INSERT INTO "FinancialAccount"(id,"userId",kind,name,"baseCurrency","updatedAt") VALUES($1,$2,'MANUAL','USD account','USD',now())"#).bind(&account).bind(&owner).execute(&pool).await.unwrap();
    sqlx::query(r#"INSERT INTO "FinancialAccount"(id,"userId",kind,name,"walletId","updatedAt") VALUES($1,$2,'WALLET','Linked wallet',$3,now())"#).bind(&linked).bind(&owner).bind(&wallet).execute(&pool).await.unwrap();
    sqlx::query(r#"INSERT INTO "Holding"(id,"accountId","assetClass",name,quantity,"currentValue","updatedAt") VALUES($1,$2,'CASH','USD cash',1,100.1234567891,now())"#).bind(uuid::Uuid::new_v4().to_string()).bind(&account).execute(&pool).await.unwrap();
    sqlx::query(r#"INSERT INTO "Contribution"(id,"accountId",kind,amount,currency,"fxRateToBrl","occurredAt") VALUES($1,$2,'DEPOSIT',10,'USD',3.12345678,'2020-01-02 02:00:00')"#).bind(uuid::Uuid::new_v4().to_string()).bind(&account).execute(&pool).await.unwrap();
    sqlx::query(r#"INSERT INTO wallet_assets(id,"walletId","assetKey",symbol,amount,"isNative",decimals,"updatedAt") VALUES($1,$2,'native','ETH',0.123456789012345678,true,18,now())"#).bind(uuid::Uuid::new_v4().to_string()).bind(&wallet).execute(&pool).await.unwrap();
    let before = repo.load(&owner).await.unwrap();
    assert_eq!(before.positions.len(), 2);
    assert_eq!(before.contributions.len(), 1);
    // Historical timestamp is converted with Sao Paulo DST rules, not today's offset.
    assert_eq!(before.contributions[0].date.to_string(), "2020-01-01");
    let saved = service.capture(&owner, Utc::now()).await.unwrap();
    assert!(saved.snapshot.complete);
    assert_eq!(saved.snapshot.positions.len(), 2);
    assert_eq!(
        saved.snapshot.net_contributions_brl.as_deref(),
        Some("31.2345678")
    );
    assert_eq!(saved.snapshot.contributions[0].source, "RECORDED");
    assert!(repo.load(&other).await.unwrap().positions.is_empty());
    assert!(service.history(&other).await.unwrap().is_empty());
    sqlx::query(r#"UPDATE "Holding" SET "currentValue"=999 WHERE "accountId"=$1"#)
        .bind(&account)
        .execute(&pool)
        .await
        .unwrap();
    let history = service.history(&owner).await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(
        history[0].snapshot.known_value_brl,
        saved.snapshot.known_value_brl
    );
    assert_ne!(
        service
            .current(&owner, Utc::now())
            .await
            .unwrap()
            .known_value_brl,
        saved.snapshot.known_value_brl
    );
    let rate: String = sqlx::query_scalar(
        r#"SELECT "fxRateToBrl"::text FROM "Contribution" WHERE "accountId"=$1"#,
    )
    .bind(&account)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(rate, "3.12345678");
    let count:i64=sqlx::query_scalar(r#"SELECT count(*) FROM "Contribution" c JOIN "FinancialAccount" a ON a.id=c."accountId" WHERE a."userId"=$1"#).bind(&owner).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1);
    // Retained assets from an incomplete synchronization may never imply a complete total.
    sqlx::query(r#"UPDATE wallets SET "lastSyncComplete"=false WHERE id=$1"#)
        .bind(&wallet)
        .execute(&pool)
        .await
        .unwrap();
    let partial = service.current(&owner, Utc::now()).await.unwrap();
    assert!(!partial.complete);
    assert!(partial.result_brl.is_none());
    sqlx::query("DELETE FROM users WHERE id=$1 OR id=$2")
        .bind(&owner)
        .bind(&other)
        .execute(&pool)
        .await
        .unwrap();
}
