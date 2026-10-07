use application::tracking::*;
use astra_infrastructure::{postgres_pool, tracking::SqlxTrackingRepository};

#[tokio::test]
#[ignore = "Requires TEST_DATABASE_URL pointing to an isolated database with the Prisma schema"]
async fn atomic_upserts_identity_precision_pruning_and_ownership() {
    let pool = postgres_pool(&std::env::var("TEST_DATABASE_URL").unwrap()).unwrap();
    let repo = SqlxTrackingRepository::new(pool.clone());
    let owner = uuid::Uuid::new_v4().to_string();
    let wallet = uuid::Uuid::new_v4().to_string();
    sqlx::query(r#"INSERT INTO users (id,email,"authId","updatedAt") VALUES ($1,$1,$1,now())"#)
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(r#"INSERT INTO wallets (id,address,chain,"userId","updatedAt") VALUES ($1,'0x0000000000000000000000000000000000000001','ETHEREUM',$2,now())"#).bind(&wallet).bind(&owner).execute(&pool).await.unwrap();
    let tiny = "0.000000000000000000000001";
    let mut w = TrackedWallet {
        wallet_id: wallet.clone(),
        address: "address".into(),
        chain: "ETHEREUM".into(),
        total_usd: Some("2".into()),
        complete: true,
        valuation_complete: true,
        updated_at: "2026-09-29T10:00:00.123Z".into(),
        assets: vec![],
    };
    for key in [
        None,
        Some("0x0000000000000000000000000000000000000001"),
        Some("0x0000000000000000000000000000000000000002"),
    ] {
        w.assets.push(TrackedAsset {
            symbol: "SAME".into(),
            name: "Same symbol, distinct identity".into(),
            amount: tiny.into(),
            price_usd: None,
            value_usd: Some("0.000000000000000000000000001".into()),
            contract_address: key.map(Into::into),
            is_native: key.is_none(),
            decimals: 24,
        });
    }
    repo.save(&owner, &w, &[], true).await.unwrap();
    let before:Vec<(String,String,String,String)>=sqlx::query_as(r#"SELECT id,"assetKey",amount::text,"usdValue"::text FROM wallet_assets WHERE "walletId"=$1 ORDER BY "assetKey""#).bind(&wallet).fetch_all(&pool).await.unwrap();
    assert_eq!(before.len(), 3);
    assert!(before.iter().all(|row| row.2.trim_end_matches('0') == tiny));
    assert!(
        before
            .iter()
            .all(|row| row.3.trim_end_matches('0') == "0.000000000000000000000000001")
    );
    repo.save(&owner, &w, &[], true).await.unwrap();
    let after: Vec<String> = sqlx::query_scalar(
        r#"SELECT id FROM wallet_assets WHERE "walletId"=$1 ORDER BY "assetKey""#,
    )
    .bind(&wallet)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        after,
        before.iter().map(|r| r.0.clone()).collect::<Vec<_>>()
    );
    assert!(matches!(
        repo.save("intruder", &w, &[], true).await,
        Err(TrackingError::NotFound)
    ));
    // On a partial page, delete only the observed zero balance, preserving the unobserved contract.
    w.assets.truncate(1);
    repo.save(
        &owner,
        &w,
        &["0x0000000000000000000000000000000000000001".into()],
        false,
    )
    .await
    .unwrap();
    let count: i64 =
        sqlx::query_scalar(r#"SELECT count(*) FROM wallet_assets WHERE "walletId"=$1"#)
            .bind(&wallet)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 2);
    // Invalid amount rolls the entire snapshot back, including earlier upserts.
    w.assets[0].amount = "invalid".into();
    assert!(repo.save(&owner, &w, &[], true).await.is_err());
    let count: i64 =
        sqlx::query_scalar(r#"SELECT count(*) FROM wallet_assets WHERE "walletId"=$1"#)
            .bind(&wallet)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 2);
    w.assets.clear();
    repo.save(&owner, &w, &[], true).await.unwrap();
    let count: i64 =
        sqlx::query_scalar(r#"SELECT count(*) FROM wallet_assets WHERE "walletId"=$1"#)
            .bind(&wallet)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
    let synced: Option<chrono::NaiveDateTime> =
        sqlx::query_scalar(r#"SELECT "lastSyncedAt" FROM wallets WHERE id=$1"#)
            .bind(&wallet)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(synced.unwrap().and_utc().timestamp_millis(), 1790676000123);
    sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(&owner)
        .execute(&pool)
        .await
        .unwrap();
}
