use application::UserService;
use astra_infrastructure::{postgres_pool, user::SqlxUserRepository};
use domain::{AuthIdentity, NewUser, RepositoryError, UserRepository, UserUpdate};
use std::sync::Arc;

#[tokio::test]
#[ignore = "Requires TEST_DATABASE_URL pointing to an isolated database with the Prisma schema"]
async fn prisma_user_contract_and_concurrent_authentication() {
    let url = std::env::var("TEST_DATABASE_URL").expect("set TEST_DATABASE_URL");
    let pool = postgres_pool(&url).unwrap();
    let repository = Arc::new(SqlxUserRepository::new(pool.clone()));
    let service = UserService::new(repository.clone());
    let id = format!("rust-test-{}", uuid::Uuid::new_v4());
    let user = service
        .create(NewUser {
            id: Some(id.clone()),
            email: format!("{id}@example.com"),
            name: Some("Alice".into()),
            auth_id: format!("auth-{id}"),
        })
        .await
        .unwrap();
    assert_eq!(user.id, id); // TEXT, not PostgreSQL UUID; matches demo_user_zora.
    assert_eq!(
        service.find_by_email(&user.email).await.unwrap(),
        Some(user.clone())
    );
    assert_eq!(service.find_by_id(&id).await.unwrap(), Some(user.clone()));
    assert_eq!(user.created_at, user.updated_at);
    let updated = service
        .update(
            &id,
            UserUpdate {
                email: Some(format!("updated-{id}@example.com")),
                name: None,
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.name.as_deref(), Some("Alice"));
    assert_eq!(updated.created_at, user.created_at);
    assert!(updated.updated_at >= user.updated_at);
    let cleared = service
        .update(
            &id,
            UserUpdate {
                name: Some(None),
                email: None,
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(cleared.name, None);
    assert_eq!(cleared.email, updated.email);
    assert!(
        service
            .find_by_id("missing-rust-test-user")
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        service
            .update("missing-rust-test-user", UserUpdate::default())
            .await
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        service
            .create(NewUser {
                id: None,
                email: updated.email.clone(),
                name: None,
                auth_id: format!("duplicate-{id}"),
            })
            .await,
        Err(RepositoryError::Conflict)
    ));
    let identity = AuthIdentity {
        auth_id: user.auth_id.clone(),
        email: Some("".into()),
    };
    let authenticated = service.authenticate(identity).await.unwrap();
    assert_eq!(authenticated.id, id);
    assert_eq!(
        authenticated.email,
        format!("user-{}@supabase.local", user.auth_id)
    );
    let new_auth_id = format!("new-auth-{id}");
    let identity = AuthIdentity {
        auth_id: new_auth_id.clone(),
        email: None,
    };
    let (first, second) = tokio::join!(
        service.authenticate(identity.clone()),
        service.authenticate(identity)
    );
    assert_eq!(first.unwrap().id, second.unwrap().id);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users WHERE \"authId\" = $1")
        .bind(&new_auth_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    let renamed = repository
        .update(
            &id,
            UserUpdate {
                email: None,
                name: Some(Some("Renamed".into())),
            },
        )
        .await
        .unwrap()
        .unwrap();
    let preserved = service
        .authenticate(AuthIdentity {
            auth_id: user.auth_id.clone(),
            email: Some(updated.email),
        })
        .await
        .unwrap();
    assert_eq!(preserved.name, renamed.name);
    sqlx::query("DELETE FROM users WHERE \"authId\" = $1 OR \"authId\" = $2")
        .bind(&user.auth_id)
        .bind(new_auth_id)
        .execute(&pool)
        .await
        .unwrap();
}

#[test]
fn rejects_supavisor_transaction_pooler() {
    assert!(postgres_pool("postgres://astra:astra@localhost:6543/astra").is_err());
    assert!(postgres_pool("not a url").is_err());
}
