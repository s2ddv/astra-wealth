use domain::watchlist::*;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sqlx::PgPool;
#[derive(Clone)]
pub struct SqlxWatchlistRepository {
    pool: PgPool,
}
impl SqlxWatchlistRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
fn error(error: sqlx::Error) -> WatchlistError {
    if error
        .as_database_error()
        .is_some_and(|e| e.is_unique_violation())
    {
        if error.as_database_error().and_then(|e| e.constraint())
            == Some("watchlist_items_watchlistId_coinId_key")
        {
            WatchlistError::ItemConflict
        } else {
            WatchlistError::NameConflict
        }
    } else {
        WatchlistError::Unavailable(Box::new(error))
    }
}
fn decode<T: DeserializeOwned>(value: Value) -> Result<T, WatchlistError> {
    serde_json::from_value(value).map_err(|e| WatchlistError::Unavailable(Box::new(e)))
}
impl WatchlistRepository for SqlxWatchlistRepository {
    fn list<'a>(&'a self, owner: &'a str) -> WatchlistFuture<'a, Vec<Watchlist>> {
        Box::pin(async move {
            sqlx::query_scalar::<_,Value>(r#"SELECT jsonb_build_object('id',w.id,'name',w.name,'userId',w."userId",'createdAt',to_char(w."createdAt",'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"'),'updatedAt',to_char(w."updatedAt",'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"'),'items',COALESCE((SELECT jsonb_agg(jsonb_build_object('id',i.id,'coinId',i."coinId",'addedAt',to_char(i."addedAt",'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"')) ORDER BY i."addedAt",i.id) FROM watchlist_items i WHERE i."watchlistId"=w.id),'[]'::jsonb)) FROM watchlists w WHERE w."userId"=$1 ORDER BY w."createdAt" DESC,w.id"#).bind(owner).fetch_all(&self.pool).await.map_err(error)?.into_iter().map(decode).collect()
        })
    }
    fn create<'a>(&'a self, owner: &'a str, name: String) -> WatchlistFuture<'a, Watchlist> {
        Box::pin(async move {
            decode(sqlx::query_scalar::<_,Value>(r#"INSERT INTO watchlists AS w (id,name,"userId","updatedAt") VALUES ($1,$2,$3,timezone('UTC',now())) RETURNING jsonb_build_object('id',w.id,'name',w.name,'userId',w."userId",'items','[]'::jsonb,'createdAt',to_char(w."createdAt",'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"'),'updatedAt',to_char(w."updatedAt",'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"'))"#).bind(uuid::Uuid::new_v4().to_string()).bind(name).bind(owner).fetch_one(&self.pool).await.map_err(error)?)
        })
    }
    fn remove<'a>(&'a self, owner: &'a str, id: &'a str) -> WatchlistFuture<'a, ()> {
        Box::pin(async move {
            if sqlx::query(r#"DELETE FROM watchlists WHERE id=$1 AND "userId"=$2"#)
                .bind(id)
                .bind(owner)
                .execute(&self.pool)
                .await
                .map_err(error)?
                .rows_affected()
                == 0
            {
                return Err(WatchlistError::NotFound);
            }
            Ok(())
        })
    }
    fn add_item<'a>(
        &'a self,
        owner: &'a str,
        id: &'a str,
        coin: String,
    ) -> WatchlistFuture<'a, WatchlistItem> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(error)?;
            if sqlx::query_scalar::<_, String>(
                r#"SELECT id FROM watchlists WHERE id=$1 AND "userId"=$2 FOR UPDATE"#,
            )
            .bind(id)
            .bind(owner)
            .fetch_optional(&mut *tx)
            .await
            .map_err(error)?
            .is_none()
            {
                return Err(WatchlistError::NotFound);
            }
            let value=sqlx::query_scalar::<_,Value>(r#"INSERT INTO watchlist_items AS i (id,"coinId","watchlistId") VALUES ($1,$2,$3) RETURNING jsonb_build_object('id',i.id,'coinId',i."coinId",'addedAt',to_char(i."addedAt",'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"'))"#).bind(uuid::Uuid::new_v4().to_string()).bind(coin).bind(id).fetch_one(&mut *tx).await.map_err(error)?;
            let item = decode(value)?;
            tx.commit().await.map_err(error)?;
            Ok(item)
        })
    }
    fn remove_item<'a>(
        &'a self,
        owner: &'a str,
        id: &'a str,
        coin: &'a str,
    ) -> WatchlistFuture<'a, ()> {
        Box::pin(async move {
            if sqlx::query(r#"DELETE FROM watchlist_items i USING watchlists w WHERE w.id=i."watchlistId" AND w.id=$1 AND w."userId"=$2 AND i."coinId"=$3"#).bind(id).bind(owner).bind(coin).execute(&self.pool).await.map_err(error)?.rows_affected()==0 {return Err(WatchlistError::ItemNotFound);}
            Ok(())
        })
    }
}
