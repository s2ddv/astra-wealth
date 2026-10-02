use astra_infrastructure::{postgres_pool, watchlist::SqlxWatchlistRepository};
use domain::watchlist::*;
#[tokio::test]
#[ignore = "Requires isolated TEST_DATABASE_URL with official Prisma migrations"]
async fn watchlists_preserve_contract_and_owner_isolation() {
    let pool = postgres_pool(&std::env::var("TEST_DATABASE_URL").unwrap()).unwrap();
    let repo = SqlxWatchlistRepository::new(pool.clone());
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
    let list = repo.create(&owner, "Favorites".into()).await.unwrap();
    assert!(list.items.is_empty());
    assert_eq!(list.user_id, owner);
    assert!(matches!(
        repo.create(&owner, "Favorites".into()).await,
        Err(WatchlistError::NameConflict)
    ));
    assert!(repo.create(&other, "Favorites".into()).await.is_ok());
    assert!(matches!(
        repo.add_item(&other, &list.id, "bitcoin".into()).await,
        Err(WatchlistError::NotFound)
    ));
    let item = repo
        .add_item(&owner, &list.id, "bitcoin".into())
        .await
        .unwrap();
    assert_eq!(item.coin_id, "bitcoin");
    assert!(matches!(
        repo.add_item(&owner, &list.id, "bitcoin".into()).await,
        Err(WatchlistError::ItemConflict)
    ));
    assert!(matches!(
        repo.remove_item(&other, &list.id, "bitcoin").await,
        Err(WatchlistError::ItemNotFound)
    ));
    assert!(matches!(
        repo.remove(&other, &list.id).await,
        Err(WatchlistError::NotFound)
    ));
    let reopened = SqlxWatchlistRepository::new(pool.clone())
        .list(&owner)
        .await
        .unwrap();
    assert_eq!(reopened.len(), 1);
    assert_eq!(reopened[0].items[0].id, item.id);
    let json = serde_json::to_value(&reopened[0]).unwrap();
    assert_eq!(json.as_object().unwrap().len(), 6);
    assert_eq!(json["items"][0].as_object().unwrap().len(), 3);
    repo.remove_item(&owner, &list.id, "bitcoin").await.unwrap();
    assert!(matches!(
        repo.remove_item(&owner, &list.id, "bitcoin").await,
        Err(WatchlistError::ItemNotFound)
    ));
    repo.add_item(&owner, &list.id, "ethereum".into())
        .await
        .unwrap();
    repo.remove(&owner, &list.id).await.unwrap();
    let count: i64 =
        sqlx::query_scalar(r#"SELECT count(*) FROM watchlist_items WHERE "watchlistId"=$1"#)
            .bind(&list.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
    sqlx::query("DELETE FROM users WHERE id=$1 OR id=$2")
        .bind(owner)
        .bind(other)
        .execute(&pool)
        .await
        .unwrap();
}
