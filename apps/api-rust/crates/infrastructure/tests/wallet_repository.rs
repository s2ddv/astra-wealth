use domain::wallet::{Chain, NewWallet, WalletRepository, WalletRepositoryError};
use zora_infrastructure::{postgres_pool, wallet::SqlxWalletRepository};

#[tokio::test]
#[ignore = "Requires TEST_DATABASE_URL pointing to an isolated database with the Prisma schema"]
async fn wallet_ownership_conflicts_and_exact_asset_amounts() {
    let pool =
        postgres_pool(&std::env::var("TEST_DATABASE_URL").expect("set TEST_DATABASE_URL")).unwrap();
    let repo = SqlxWalletRepository::new(pool.clone());
    let owner = uuid::Uuid::new_v4().to_string();
    let other = uuid::Uuid::new_v4().to_string();
    for id in [&owner, &other] {
        sqlx::query(r#"INSERT INTO users (id,email,"authId","updatedAt") VALUES ($1,$2,$1,now())"#)
            .bind(id)
            .bind(format!("{id}@example.com"))
            .execute(&pool)
            .await
            .unwrap();
    }
    let input = || NewWallet {
        address: "0xde709f2102306220921060314715629080e2fb77".into(),
        chain: Chain::Ethereum,
        nickname: None,
    };
    let wallet = repo.create(&owner, input()).await.unwrap();
    assert_eq!(wallet.user_id, owner);
    assert!(matches!(
        repo.create(&owner, input()).await,
        Err(WalletRepositoryError::Conflict)
    ));
    let other_wallet = repo.create(&other, input()).await.unwrap();
    assert_eq!(other_wallet.user_id, other);
    assert!(repo.find_by_id(&wallet.id, &other).await.unwrap().is_none());
    assert_eq!(
        repo.update_nickname(&wallet.id, &other, "intruder")
            .await
            .unwrap(),
        0
    );
    assert_eq!(repo.delete(&wallet.id, &other).await.unwrap(), 0);
    sqlx::query(r#"INSERT INTO wallet_assets (id,symbol,amount,"walletId","updatedAt") VALUES ($1,'ETH',99999999999999999999.123456789123456789,$2,now())"#)
        .bind(uuid::Uuid::new_v4().to_string()).bind(&wallet.id).execute(&pool).await.unwrap();
    let listed = repo.find_by_user_id(&owner).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, wallet.id);
    assert_eq!(
        listed[0].assets[0].amount,
        "99999999999999999999.123456789123456789"
    );
    let other_list = repo.find_by_user_id(&other).await.unwrap();
    assert_eq!(other_list.len(), 1);
    assert!(other_list[0].assets.is_empty());
    assert_eq!(
        repo.update_nickname(&wallet.id, &owner, "Savings")
            .await
            .unwrap(),
        1
    );
    let updated = repo.find_by_id(&wallet.id, &owner).await.unwrap().unwrap();
    assert_eq!(updated.nickname.as_deref(), Some("Savings"));
    assert_eq!(updated.assets, listed[0].assets);
    assert_eq!(repo.delete(&wallet.id, &owner).await.unwrap(), 1);
    assert!(repo.find_by_id(&wallet.id, &owner).await.unwrap().is_none());
    let remaining: i64 =
        sqlx::query_scalar(r#"SELECT count(*) FROM wallet_assets WHERE "walletId"=$1"#)
            .bind(&wallet.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(remaining, 0);
    sqlx::query("DELETE FROM users WHERE id=$1 OR id=$2")
        .bind(owner)
        .bind(other)
        .execute(&pool)
        .await
        .unwrap();
}
