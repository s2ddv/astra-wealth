use application::tracking::*;
use sqlx::PgPool;
pub struct SqlxTrackingRepository {
    pool: PgPool,
}
impl SqlxTrackingRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
fn error(_: sqlx::Error) -> TrackingError {
    TrackingError::StorageUnavailable
}
impl TrackingRepository for SqlxTrackingRepository {
    fn save<'a>(
        &'a self,
        user: &'a str,
        wallet: &'a TrackedWallet,
        observed: &'a [String],
        complete: bool,
    ) -> TrackingFuture<'a, ()> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(error)?;
            let owner = sqlx::query_scalar::<_, String>(
                r#"SELECT id FROM wallets WHERE id=$1 AND "userId"=$2 FOR UPDATE"#,
            )
            .bind(&wallet.wallet_id)
            .bind(user)
            .fetch_optional(&mut *tx)
            .await
            .map_err(error)?;
            if owner.is_none() {
                return Err(TrackingError::NotFound);
            }
            let updated = chrono::DateTime::parse_from_rfc3339(&wallet.updated_at)
                .map_err(|_| TrackingError::StorageUnavailable)?
                .naive_utc();
            let mut keys = Vec::new();
            for asset in &wallet.assets {
                sqlx::query(r#"INSERT INTO wallet_assets(id,"walletId","assetKey",symbol,name,"contractAddress","isNative",decimals,amount,"usdValue","updatedAt") VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9::text::numeric,$10::text::numeric,$11) ON CONFLICT("walletId","assetKey") DO UPDATE SET symbol=EXCLUDED.symbol,name=EXCLUDED.name,"contractAddress"=EXCLUDED."contractAddress","isNative"=EXCLUDED."isNative",decimals=EXCLUDED.decimals,amount=EXCLUDED.amount,"usdValue"=EXCLUDED."usdValue","updatedAt"=EXCLUDED."updatedAt""#)
            .bind(uuid::Uuid::new_v4().to_string()).bind(&wallet.wallet_id).bind(asset.key()).bind(&asset.symbol).bind(&asset.name).bind(&asset.contract_address).bind(asset.is_native).bind(i32::from(asset.decimals)).bind(&asset.amount).bind(&asset.value_usd).bind(updated).execute(&mut *tx).await.map_err(error)?;
                keys.push(asset.key().to_owned());
            }
            sqlx::query(r#"DELETE FROM wallet_assets WHERE "walletId"=$1 AND NOT ("assetKey"=ANY($2)) AND ($3 OR "assetKey"=ANY($4))"#).bind(&wallet.wallet_id).bind(&keys).bind(complete).bind(observed).execute(&mut *tx).await.map_err(error)?;
            sqlx::query(r#"UPDATE wallets SET "lastSyncedAt"=$3,"updatedAt"=$3,"lastSyncComplete"=$4,"lastValuationComplete"=$5 WHERE id=$1 AND "userId"=$2"#).bind(&wallet.wallet_id).bind(user).bind(updated).bind(complete && wallet.complete).bind(complete && wallet.valuation_complete).execute(&mut *tx).await.map_err(error)?;
            tx.commit().await.map_err(error)
        })
    }
}
