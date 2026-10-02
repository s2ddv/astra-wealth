use astra_infrastructure::{accounts::SqlxAccountRepository, postgres_pool};
use domain::accounts::*;

#[tokio::test]
#[ignore = "Requires isolated TEST_DATABASE_URL with official Prisma migrations"]
async fn accounts_persist_exact_records_and_enforce_ownership() {
    let pool = postgres_pool(&std::env::var("TEST_DATABASE_URL").unwrap()).unwrap();
    let repo = SqlxAccountRepository::new(pool.clone());
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
    let account = repo
        .create(
            &owner,
            NewAccount {
                name: "Savings".into(),
                institution_name: None,
                kind: "MANUAL".into(),
                base_currency: "BRL".into(),
                wallet_id: None,
            },
        )
        .await
        .unwrap();
    let contribution = || NewContribution {
        kind: "DEPOSIT".into(),
        amount: "999999999999999999.1234567891".into(),
        currency: "USD".into(),
        fx_rate_to_brl: Some("5.12345678".into()),
        occurred_at: "2020-01-01T02:00:00+02:00".into(),
        note: Some("Explicit deposit".into()),
    };
    let holding = || NewHolding {
        asset_class: "CASH".into(),
        symbol: None,
        name: "Cash".into(),
        quantity: "1.0000000001".into(),
        unit_price: None,
        current_value: "123.1234567891".into(),
        due_date: None,
    };
    assert!(matches!(
        repo.find(&other, &account.id).await,
        Err(AccountError::NotFound)
    ));
    assert!(matches!(
        repo.add_contribution(&other, &account.id, contribution())
            .await,
        Err(AccountError::NotFound)
    ));
    assert!(matches!(
        repo.save_holding(&other, &account.id, None, holding())
            .await,
        Err(AccountError::NotFound)
    ));
    assert!(matches!(
        repo.remove(&other, &account.id).await,
        Err(AccountError::NotFound)
    ));
    let deposit = repo
        .add_contribution(&owner, &account.id, contribution())
        .await
        .unwrap();
    assert_eq!(deposit.amount, "999999999999999999.1234567891");
    assert_eq!(deposit.occurred_at, "2020-01-01T00:00:00.000Z");
    assert_eq!(deposit.origin, "MANUAL");
    assert!(
        repo.find(&owner, &account.id)
            .await
            .unwrap()
            .holdings
            .is_empty()
    );
    let position = repo
        .save_holding(&owner, &account.id, None, holding())
        .await
        .unwrap();
    let reopened = SqlxAccountRepository::new(pool.clone())
        .find(&owner, &account.id)
        .await
        .unwrap();
    assert_eq!(reopened.holdings[0].quantity, "1.0000000001");
    assert_eq!(reopened.contributions.len(), 1);
    assert_eq!(repo.list(&other).await.unwrap().len(), 0);
    let mut change = holding();
    change.current_value = "50".into();
    let changed = repo
        .save_holding(&owner, &account.id, Some(&position.id), change)
        .await
        .unwrap();
    assert_eq!(changed.current_value, "50.0000000000");
    assert!(matches!(
        repo.remove_record(&other, &account.id, &deposit.id, false)
            .await,
        Err(AccountError::NotFound)
    ));
    repo.remove_record(&owner, &account.id, &deposit.id, false)
        .await
        .unwrap();
    assert_eq!(
        repo.find(&owner, &account.id).await.unwrap().holdings.len(),
        1
    );
    let wallet = uuid::Uuid::new_v4().to_string();
    sqlx::query(r#"INSERT INTO wallets(id,address,chain,"userId","updatedAt") VALUES($1,'0xde709f2102306220921060314715629080e2fb77','ETHEREUM',$2,now())"#).bind(&wallet).bind(&owner).execute(&pool).await.unwrap();
    let linked = || NewAccount {
        name: "Crypto".into(),
        institution_name: None,
        kind: "WALLET".into(),
        base_currency: "USD".into(),
        wallet_id: Some(wallet.clone()),
    };
    assert!(matches!(
        repo.create(&other, linked()).await,
        Err(AccountError::NotFound)
    ));
    let crypto = repo.create(&owner, linked()).await.unwrap();
    assert!(matches!(
        repo.create(&owner, linked()).await,
        Err(AccountError::Conflict)
    ));
    assert!(matches!(
        repo.save_holding(&owner, &crypto.id, None, holding()).await,
        Err(AccountError::Validation(_))
    ));
    assert!(
        repo.find(&owner, &crypto.id)
            .await
            .unwrap()
            .contributions
            .is_empty()
    );
    use domain::wallet::{WalletRepository, WalletRepositoryError};
    let wallets = astra_infrastructure::wallet::SqlxWalletRepository::new(pool.clone());
    assert!(matches!(
        wallets.delete(&wallet, &owner).await,
        Err(WalletRepositoryError::LinkedAccount)
    ));
    repo.remove(&owner, &crypto.id).await.unwrap();
    let wallet_count: i64 = sqlx::query_scalar("SELECT count(*) FROM wallets WHERE id=$1")
        .bind(&wallet)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        wallet_count, 1,
        "Deleting account preserves its linked wallet"
    );
    repo.remove(&owner, &account.id).await.unwrap();
    let child_count: i64 =
        sqlx::query_scalar(r#"SELECT count(*) FROM "Holding" WHERE "accountId"=$1"#)
            .bind(&account.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(child_count, 0);
    sqlx::query("DELETE FROM users WHERE id=$1 OR id=$2")
        .bind(owner)
        .bind(other)
        .execute(&pool)
        .await
        .unwrap();
}
