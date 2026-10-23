use application::usd_snapshots::{UsdSnapshotRepository, UsdSnapshotService};
use astra_infrastructure::{postgres_pool, usd_snapshots::SqlxUsdSnapshotRepository};
use std::sync::Arc;
#[tokio::test]
#[ignore = "Requires disposable TEST_DATABASE_URL"]
async fn legacy_rows_remain_readable_without_rewriting_usd_history() {
    let url = std::env::var("TEST_DATABASE_URL").unwrap();
    let parsed = url::Url::parse(&url).unwrap();
    assert_eq!(parsed.host_str(), Some("127.0.0.1"));
    assert_eq!(parsed.path(), "/astra_isolation_test");
    let pool = postgres_pool(&url).unwrap();
    let owner = uuid::Uuid::new_v4().to_string();
    let other = uuid::Uuid::new_v4().to_string();
    for id in [&owner, &other] {
        sqlx::query(r#"INSERT INTO users(id,email,"authId","updatedAt") VALUES($1,$2,$1,now())"#)
            .bind(id)
            .bind(format!("{id}@test.invalid"))
            .execute(&pool)
            .await
            .unwrap();
    }
    let legacy = uuid::Uuid::new_v4().to_string();
    sqlx::query(r#"INSERT INTO portfolio_snapshots(id,"userId","totalUsd",source,"createdAt") VALUES($1,$2,123.40,'SCHEDULED','2020-01-01')"#).bind(&legacy).bind(&owner).execute(&pool).await.unwrap();
    let repo = Arc::new(SqlxUsdSnapshotRepository::new(pool.clone()));
    let service = UsdSnapshotService::new(repo.clone());
    assert_eq!(
        service.list(&owner, 30).await.unwrap()[0].total_usd,
        "123.4"
    );
    assert!(service.list(&other, 30).await.unwrap().is_empty());
    let fresh = service.create(&owner, "9999999999999999.99").await.unwrap();
    assert_eq!(fresh.total_usd, "9999999999999999.99");
    assert_eq!(repo.list(&owner, 1).await.unwrap()[0].id, fresh.id);
    assert!(
        service
            .create(&owner, "9999999999999999.995")
            .await
            .is_err()
    );
    assert_eq!(service.list(&owner, 100).await.unwrap().len(), 2);
    let source: String =
        sqlx::query_scalar("SELECT source::text FROM portfolio_snapshots WHERE id=$1")
            .bind(&legacy)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(source, "SCHEDULED");
    let source: String =
        sqlx::query_scalar("SELECT source::text FROM portfolio_snapshots WHERE id=$1")
            .bind(&fresh.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(source, "WALLET_REFRESH");
    for id in [&owner, &other] {
        sqlx::query("DELETE FROM users WHERE id=$1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }
}
