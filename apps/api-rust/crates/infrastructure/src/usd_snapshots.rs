use application::usd_snapshots::*;
use serde_json::Value;
use sqlx::PgPool;
#[derive(Clone)]
pub struct SqlxUsdSnapshotRepository {
    pool: PgPool,
}
impl SqlxUsdSnapshotRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
fn decode(value: Value) -> Result<UsdSnapshot, SnapshotError> {
    serde_json::from_value(value).map_err(|_| SnapshotError::Unavailable)
}
impl UsdSnapshotRepository for SqlxUsdSnapshotRepository {
    fn list<'a>(&'a self, owner: &'a str, limit: i64) -> SnapshotFuture<'a, Vec<UsdSnapshot>> {
        Box::pin(async move {
            sqlx::query_scalar::<_,Value>(r#"SELECT jsonb_build_object('id',id,'totalUsd',trim_scale("totalUsd")::text,'userId',"userId",'createdAt',to_char("createdAt",'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"')) FROM portfolio_snapshots WHERE "userId"=$1 ORDER BY "createdAt" DESC,id LIMIT $2"#)
                .bind(owner).bind(limit).fetch_all(&self.pool).await.map_err(|_| SnapshotError::Unavailable)?.into_iter().map(decode).collect()
        })
    }
    fn create<'a>(&'a self, owner: &'a str, total: String) -> SnapshotFuture<'a, UsdSnapshot> {
        Box::pin(async move {
            decode(sqlx::query_scalar::<_,Value>(r#"INSERT INTO portfolio_snapshots(id,"userId","totalUsd",source) VALUES($1,$2,$3::numeric,'WALLET_REFRESH') RETURNING jsonb_build_object('id',id,'totalUsd',trim_scale("totalUsd")::text,'userId',"userId",'createdAt',to_char("createdAt",'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"'))"#)
                .bind(uuid::Uuid::new_v4().to_string()).bind(owner).bind(total).fetch_one(&self.pool).await.map_err(|_| SnapshotError::Unavailable)?)
        })
    }
}
