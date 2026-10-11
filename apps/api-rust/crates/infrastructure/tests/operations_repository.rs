use application::{
    operations::*,
    portfolio::{PortfolioRepository, PositionKind},
};
use astra_infrastructure::{
    accounts::SqlxAccountRepository, operations::SqlxOperationRepository,
    portfolio::SqlxPortfolioRepository, postgres_pool,
};
use domain::accounts::*;
fn input(kind: &str, holding: Option<&str>) -> NewOperation {
    let trade = ["BUY", "SELL", "OPENING"].contains(&kind);
    NewOperation {
        request_id: uuid::Uuid::new_v4().to_string(),
        kind: kind.into(),
        holding_id: holding.map(str::to_owned),
        destination_account_id: None,
        replaces_id: None,
        market: trade.then(|| "BR".into()),
        quantity: trade.then(|| "10".into()),
        amount: Some("100.1234567891".into()),
        fees: if ["BUY", "SELL", "INCOME"].contains(&kind) {
            Some("0".into())
        } else {
            None
        },
        currency: "BRL".into(),
        occurred_at: "2020-01-01T00:00:00Z".into(),
        note: None,
    }
}
#[tokio::test]
#[ignore = "Requires disposable TEST_DATABASE_URL with official migrations"]
async fn persistence_isolation_replay_transfers_and_concurrent_retries() {
    let pool = postgres_pool(&std::env::var("TEST_DATABASE_URL").unwrap()).unwrap();
    let accounts = SqlxAccountRepository::new(pool.clone());
    let operations = SqlxOperationRepository::new(pool.clone());
    let owner = uuid::Uuid::new_v4().to_string();
    let other = uuid::Uuid::new_v4().to_string();
    for user in [&owner, &other] {
        sqlx::query(r#"INSERT INTO users(id,email,"authId","updatedAt") VALUES($1,$2,$1,now())"#)
            .bind(user)
            .bind(format!("{user}@test.invalid"))
            .execute(&pool)
            .await
            .unwrap();
    }
    let mut ids = Vec::new();
    for (user, currency) in [
        (&owner, "BRL"),
        (&owner, "BRL"),
        (&other, "BRL"),
        (&owner, "USD"),
    ] {
        ids.push(
            accounts
                .create(
                    user,
                    NewAccount {
                        name: "Ledger".into(),
                        kind: "MANUAL".into(),
                        institution_name: None,
                        base_currency: currency.into(),
                        wallet_id: None,
                    },
                )
                .await
                .unwrap()
                .id,
        );
    }
    let mut holdings = Vec::new();
    for (user, account) in [(&owner, &ids[0]), (&owner, &ids[1]), (&other, &ids[2])] {
        holdings.push(
            accounts
                .save_holding(
                    user,
                    account,
                    None,
                    NewHolding {
                        asset_class: "STOCK".into(),
                        symbol: Some("TEST3".into()),
                        name: "Test".into(),
                        quantity: "10".into(),
                        unit_price: Some("10".into()),
                        current_value: "100".into(),
                        due_date: None,
                    },
                )
                .await
                .unwrap()
                .id,
        );
    }
    let portfolio = SqlxPortfolioRepository::new(pool.clone());
    let before = portfolio.load(&owner).await.unwrap();
    assert!(before.contributions.is_empty());
    let values = |p: &application::portfolio::PortfolioInput| {
        p.positions
            .iter()
            .map(|p| match &p.kind {
                PositionKind::Fiat { value, .. } => value.clone(),
                _ => "unavailable".into(),
            })
            .collect::<Vec<_>>()
    };
    let mut transfer = input("TRANSFER", None);
    transfer.destination_account_id = Some(ids[1].clone());
    let moved = operations
        .append(&owner, &ids[0], transfer.clone())
        .await
        .unwrap();
    assert_eq!(
        operations.list(&owner, &ids[1]).await.unwrap()[0].id,
        moved.id
    );
    assert_eq!(
        values(&before),
        values(&portfolio.load(&owner).await.unwrap())
    );
    assert!(
        accounts
            .find(&owner, &ids[0])
            .await
            .unwrap()
            .contributions
            .is_empty()
    );
    assert!(
        portfolio
            .load(&owner)
            .await
            .unwrap()
            .contributions
            .is_empty()
    );
    for dest in [&ids[0], &ids[3]] {
        let mut bad = transfer.clone();
        bad.request_id = uuid::Uuid::new_v4().to_string();
        bad.destination_account_id = Some(dest.clone());
        assert!(matches!(
            operations.append(&owner, &ids[0], bad).await,
            Err(AccountError::Validation(_))
        ));
    }
    let mut foreign = transfer.clone();
    foreign.request_id = uuid::Uuid::new_v4().to_string();
    foreign.destination_account_id = Some(ids[2].clone());
    assert!(matches!(
        operations.append(&owner, &ids[0], foreign).await,
        Err(AccountError::NotFound)
    ));
    assert!(matches!(
        operations.list(&other, &ids[0]).await,
        Err(AccountError::NotFound)
    ));
    assert!(matches!(
        operations.list(&other, &ids[1]).await,
        Err(AccountError::NotFound)
    ));
    assert!(matches!(
        operations.append(&other, &ids[0], transfer.clone()).await,
        Err(AccountError::NotFound)
    ));
    for h in [&holdings[1], &holdings[2]] {
        assert!(matches!(
            operations
                .append(&owner, &ids[0], input("BUY", Some(h)))
                .await,
            Err(AccountError::NotFound)
        ));
    }
    assert!(operations.list(&other, &ids[2]).await.unwrap().is_empty());
    let buy = input("BUY", Some(&holdings[0]));
    let (a, b) = tokio::join!(
        operations.append(&owner, &ids[0], buy.clone()),
        operations.append(&owner, &ids[0], buy.clone())
    );
    let bought = a.unwrap();
    assert_eq!(bought.id, b.unwrap().id);
    let mut conflicting = buy.clone();
    conflicting.amount = Some("1".into());
    assert!(matches!(
        operations.append(&owner, &ids[0], conflicting).await,
        Err(AccountError::Conflict)
    ));
    let reopened = SqlxOperationRepository::new(pool.clone());
    let stored = reopened.list(&owner, &ids[0]).await.unwrap();
    assert_eq!(stored.len(), 2);
    assert_eq!(
        stored
            .iter()
            .find(|o| o.id == bought.id)
            .unwrap()
            .input
            .amount
            .as_deref(),
        Some("100.1234567891")
    );
    assert!(accounts.find(&owner, &ids[0]).await.unwrap().holdings[0].valuation_pending);
    assert!(
        portfolio
            .load(&owner)
            .await
            .unwrap()
            .positions
            .iter()
            .any(|p| p.issue.as_deref() == Some("MANUAL_VALUATION_REQUIRED"))
    );
    let mut sell = input("SELL", Some(&holdings[0]));
    sell.quantity = Some("5".into());
    sell.amount = Some("80".into());
    sell.fees = Some("1".into());
    sell.occurred_at = "2020-01-02T00:00:00Z".into();
    let sold = operations.append(&owner, &ids[0], sell).await.unwrap();
    let mut correction = buy.clone();
    correction.request_id = uuid::Uuid::new_v4().to_string();
    correction.replaces_id = Some(bought.id.clone());
    correction.amount = Some("120".into());
    let corrected = operations
        .append(&owner, &ids[0], correction.clone())
        .await
        .unwrap();
    let l = replay(reopened.list(&owner, &ids[0]).await.unwrap(), &ids[0]).unwrap();
    assert_eq!(
        l.results
            .iter()
            .find(|r| r.id == sold.id)
            .unwrap()
            .realized_result
            .as_deref(),
        Some("19")
    );
    assert!(!l.results.iter().find(|r| r.id == bought.id).unwrap().active);
    assert_eq!(l.positions[0].quantity, "5");
    assert_eq!(l.positions[0].accumulated_cost.as_deref(), Some("60"));
    let mut bad = correction.clone();
    bad.request_id = uuid::Uuid::new_v4().to_string();
    bad.replaces_id = Some(corrected.id.clone());
    bad.quantity = Some("1".into());
    assert!(matches!(
        operations.append(&owner, &ids[0], bad.clone()).await,
        Err(AccountError::Validation(_))
    ));
    assert_eq!(operations.list(&owner, &ids[0]).await.unwrap().len(), 4);
    bad.quantity = Some("10".into());
    bad.replaces_id = Some(moved.id.clone());
    assert!(matches!(
        operations.append(&other, &ids[2], bad).await,
        Err(AccountError::NotFound)
    ));
    assert!(matches!(
        accounts.remove(&owner, &ids[1]).await,
        Err(AccountError::Conflict)
    ));
    assert!(matches!(
        accounts
            .remove_record(&owner, &ids[0], &holdings[0], true)
            .await,
        Err(AccountError::Conflict)
    ));
    // Owner lock also serializes competing sells; one succeeds, one rolls back oversell.
    let mut last = input("SELL", Some(&holdings[0]));
    last.quantity = Some("5".into());
    last.occurred_at = "2020-01-03T00:00:00Z".into();
    let mut competing = last.clone();
    competing.request_id = uuid::Uuid::new_v4().to_string();
    let (a, b) = tokio::join!(
        operations.append(&owner, &ids[0], last),
        operations.append(&owner, &ids[0], competing)
    );
    assert_ne!(a.is_ok(), b.is_ok());
    // Even a browser role with table grants cannot bypass the Rust owner checks.
    sqlx::query("CREATE ROLE operations_browser_test NOLOGIN")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("GRANT USAGE ON SCHEMA public TO operations_browser_test")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "GRANT SELECT,INSERT,UPDATE,DELETE ON financial_operations TO operations_browser_test",
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SET LOCAL ROLE operations_browser_test")
        .execute(&mut *tx)
        .await
        .unwrap();
    let exposed: i64 = sqlx::query_scalar("SELECT count(*) FROM financial_operations")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(exposed, 0);
    assert_eq!(
        sqlx::query("DELETE FROM financial_operations")
            .execute(&mut *tx)
            .await
            .unwrap()
            .rows_affected(),
        0
    );
    tx.rollback().await.unwrap();
    // All data live in a disposable database, destroyed by the runner.
}
