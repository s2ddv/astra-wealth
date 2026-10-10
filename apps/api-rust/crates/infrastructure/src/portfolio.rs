use application::portfolio::*;
use chrono::{NaiveDateTime, Utc};
use sqlx::{PgPool, Row};

pub struct SqlxPortfolioRepository {
    pool: PgPool,
}
impl SqlxPortfolioRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
fn error(e: sqlx::Error) -> PortfolioError {
    tracing::error!(%e, "Portfolio storage failure");
    PortfolioError
}
fn timestamp(row: &sqlx::postgres::PgRow, name: &str) -> Result<String, PortfolioError> {
    Ok(row
        .try_get::<NaiveDateTime, _>(name)
        .map_err(error)?
        .and_utc()
        .to_rfc3339())
}
impl PortfolioRepository for SqlxPortfolioRepository {
    fn load<'a>(&'a self, owner: &'a str) -> PortfolioFuture<'a, PortfolioInput> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(error)?;
            sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
                .execute(&mut *tx)
                .await
                .map_err(error)?;
            let observed: NaiveDateTime =
                sqlx::query_scalar("SELECT transaction_timestamp() AT TIME ZONE 'UTC'")
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(error)?;
            let mut positions = Vec::new();
            let holdings = sqlx::query(r#"SELECT h.id,h.name,h."accountId",h."currentValue"::text AS value,h."assetClass"::text AS class,h."updatedAt",a."baseCurrency"::text AS currency FROM "Holding" h JOIN "FinancialAccount" a ON a.id=h."accountId" WHERE a."userId"=$1 AND a.kind != 'WALLET' ORDER BY h.id"#)
                .bind(owner).fetch_all(&mut *tx).await.map_err(error)?;
            for h in holdings {
                let class: String = h.try_get("class").map_err(error)?;
                positions.push(PositionInput {
                    id: h.try_get("id").map_err(error)?,
                    account_id: Some(h.try_get("accountId").map_err(error)?),
                    wallet_id: None,
                    name: h.try_get("name").map_err(error)?,
                    updated_at: timestamp(&h, "updatedAt")?,
                    issue: None,
                    kind: if class == "CRYPTO" {
                        PositionKind::Unsupported
                    } else {
                        PositionKind::Fiat {
                            currency: h.try_get("currency").map_err(error)?,
                            value: h.try_get("value").map_err(error)?,
                        }
                    },
                });
            }
            // Both linked and standalone wallets, once each. Never add wallet-account holdings.
            let rows = sqlx::query(r#"SELECT w.id AS wallet,w.chain::text,w."lastSyncedAt",w."lastSyncComplete",w."updatedAt" AS wallet_updated,a.id AS account,
                wa.id,COALESCE(wa.name,wa.symbol,w.nickname,w.address) AS name,wa.amount::text,wa."contractAddress",wa."isNative",wa."updatedAt"
                FROM wallets w LEFT JOIN "FinancialAccount" a ON a."walletId"=w.id AND a."userId"=w."userId"
                LEFT JOIN wallet_assets wa ON wa."walletId"=w.id WHERE w."userId"=$1 ORDER BY w.id,wa.id"#)
                .bind(owner).fetch_all(&mut *tx).await.map_err(error)?;
            for w in rows {
                let wallet: String = w.try_get("wallet").map_err(error)?;
                let synced: Option<NaiveDateTime> = w.try_get("lastSyncedAt").map_err(error)?;
                let complete: bool = w.try_get("lastSyncComplete").map_err(error)?;
                let issue = if synced.is_none() {
                    Some("NOT_SYNCED".into())
                } else if !complete {
                    Some("PARTIAL_BALANCES".into())
                } else if synced.is_some_and(|t| {
                    observed.signed_duration_since(t).num_seconds() > 86400 || t > observed
                }) {
                    Some("STALE_BALANCE".into())
                } else {
                    None
                };
                let id: Option<String> = w.try_get("id").map_err(error)?;
                let kind = if id.is_none() {
                    PositionKind::EmptyWallet
                } else {
                    let chain: String = w.try_get("chain").map_err(error)?;
                    let native: bool = w.try_get("isNative").map_err(error)?;
                    let contract: Option<String> = w.try_get("contractAddress").map_err(error)?;
                    if !native && contract.is_none() {
                        PositionKind::Unsupported
                    } else {
                        PositionKind::Crypto {
                            chain: chain.parse().map_err(|_| PortfolioError)?,
                            contract: if native { None } else { contract },
                            amount: w.try_get("amount").map_err(error)?,
                        }
                    }
                };
                let updated_at = timestamp(
                    &w,
                    if id.is_some() {
                        "updatedAt"
                    } else {
                        "wallet_updated"
                    },
                )?;
                positions.push(PositionInput {
                    id: id.unwrap_or_else(|| format!("wallet:{wallet}")),
                    wallet_id: Some(wallet),
                    account_id: w.try_get("account").map_err(error)?,
                    name: w.try_get("name").map_err(error)?,
                    updated_at,
                    issue,
                    kind,
                });
            }
            let rows = sqlx::query(r#"SELECT c.id,c.amount::text,c.currency::text,c.kind::text,c."fxRateToBrl"::text AS rate,
                (c."occurredAt" AT TIME ZONE 'UTC' AT TIME ZONE 'America/Sao_Paulo')::date AS date
                FROM "Contribution" c JOIN "FinancialAccount" a ON a.id=c."accountId" WHERE a."userId"=$1 AND c."occurredAt" <= $2 ORDER BY c.id"#)
                .bind(owner).bind(observed).fetch_all(&mut *tx).await.map_err(error)?;
            let mut contributions = Vec::new();
            for c in rows {
                contributions.push(ContributionInput {
                    id: c.try_get("id").map_err(error)?,
                    amount: c.try_get("amount").map_err(error)?,
                    currency: c.try_get("currency").map_err(error)?,
                    withdrawal: c.try_get::<String, _>("kind").map_err(error)? == "WITHDRAWAL",
                    date: c.try_get("date").map_err(error)?,
                    recorded_rate: c.try_get("rate").map_err(error)?,
                });
            }
            tx.commit().await.map_err(error)?;
            Ok(PortfolioInput {
                observed_at: observed.and_utc().to_rfc3339(),
                positions,
                contributions,
            })
        })
    }
    fn save<'a>(
        &'a self,
        owner: &'a str,
        snapshot: &'a PortfolioSnapshot,
    ) -> PortfolioFuture<'a, String> {
        Box::pin(async move {
            let id = uuid::Uuid::new_v4().to_string();
            let payload = serde_json::to_string(snapshot).map_err(|_| PortfolioError)?;
            sqlx::query(r#"INSERT INTO portfolio_valuations(id,"userId",payload,"createdAt") VALUES($1,$2,$3::text::jsonb,$4)"#)
                .bind(&id).bind(owner).bind(payload).bind(Utc::now().naive_utc()).execute(&self.pool).await.map_err(error)?;
            Ok(id)
        })
    }
    fn history<'a>(&'a self, owner: &'a str) -> PortfolioFuture<'a, Vec<SavedSnapshot>> {
        Box::pin(async move {
            let rows = sqlx::query(r#"SELECT id,payload::text FROM portfolio_valuations WHERE "userId"=$1 ORDER BY "createdAt" DESC,id DESC LIMIT 100"#).bind(owner).fetch_all(&self.pool).await.map_err(error)?;
            rows.into_iter()
                .map(|r| {
                    Ok(SavedSnapshot {
                        id: r.try_get("id").map_err(error)?,
                        snapshot: serde_json::from_str(
                            &r.try_get::<String, _>("payload").map_err(error)?,
                        )
                        .map_err(|_| PortfolioError)?,
                    })
                })
                .collect()
        })
    }
}
