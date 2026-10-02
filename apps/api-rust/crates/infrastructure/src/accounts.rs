//! Owner-scoped SQL against the official Prisma schema.
use domain::accounts::*;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};

#[derive(Clone)]
pub struct SqlxAccountRepository {
    pool: PgPool,
}
impl SqlxAccountRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
fn error(error: sqlx::Error) -> AccountError {
    if error
        .as_database_error()
        .is_some_and(|e| e.is_unique_violation())
    {
        AccountError::Conflict
    } else {
        AccountError::Unavailable(Box::new(error))
    }
}
fn decode<T: DeserializeOwned>(value: Value) -> Result<T, AccountError> {
    serde_json::from_value(value).map_err(|e| AccountError::Unavailable(Box::new(e)))
}
// SQL interpolation contains only static projections and a two-table whitelist;
// every client value uses a bind parameter.
// Explicit text projections prevent JSON numeric rounding and fix UTC timestamp semantics.
const ACCOUNT: &str = r#"to_jsonb(a) || jsonb_build_object('createdAt',to_char(a."createdAt",'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"'),'updatedAt',to_char(a."updatedAt",'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"'))"#;
const HOLDING: &str = r#"to_jsonb(h) || jsonb_build_object('quantity',h.quantity::text,'unitPrice',h."unitPrice"::text,'currentValue',h."currentValue"::text,'dueDate',to_char(h."dueDate",'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"'),'createdAt',to_char(h."createdAt",'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"'),'updatedAt',to_char(h."updatedAt",'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"'))"#;
const CONTRIBUTION: &str = r#"to_jsonb(c) || jsonb_build_object('amount',c.amount::text,'fxRateToBrl',c."fxRateToBrl"::text,'occurredAt',to_char(c."occurredAt",'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"'),'createdAt',to_char(c."createdAt",'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"'))"#;
async fn lock_account(
    tx: &mut Transaction<'_, Postgres>,
    owner: &str,
    id: &str,
) -> Result<String, AccountError> {
    sqlx::query_scalar(
        r#"SELECT kind::text FROM "FinancialAccount" WHERE id=$1 AND "userId"=$2 FOR UPDATE"#,
    )
    .bind(id)
    .bind(owner)
    .fetch_optional(&mut **tx)
    .await
    .map_err(error)?
    .ok_or(AccountError::NotFound)
}
impl AccountRepository for SqlxAccountRepository {
    fn list<'a>(&'a self, owner: &'a str) -> AccountFuture<'a, Vec<Account>> {
        Box::pin(async move {
            let query = format!(
                r#"SELECT {ACCOUNT} FROM "FinancialAccount" a WHERE a."userId"=$1 ORDER BY a."createdAt" DESC,a.id"#
            );
            sqlx::query_scalar::<_, Value>(sqlx::AssertSqlSafe(query.as_str()))
                .bind(owner)
                .fetch_all(&self.pool)
                .await
                .map_err(error)?
                .into_iter()
                .map(decode)
                .collect()
        })
    }
    fn find<'a>(&'a self, owner: &'a str, id: &'a str) -> AccountFuture<'a, AccountDetail> {
        Box::pin(async move {
            // One statement gives a consistent snapshot and scopes every child to the owned parent.
            let query = format!(
                r#"SELECT {ACCOUNT} || jsonb_build_object('holdings',COALESCE((SELECT jsonb_agg({HOLDING} ORDER BY h."createdAt",h.id) FROM "Holding" h WHERE h."accountId"=a.id),'[]'::jsonb),'contributions',COALESCE((SELECT jsonb_agg({CONTRIBUTION} ORDER BY c."occurredAt" DESC,c.id) FROM "Contribution" c WHERE c."accountId"=a.id),'[]'::jsonb)) FROM "FinancialAccount" a WHERE a.id=$1 AND a."userId"=$2"#
            );
            decode(
                sqlx::query_scalar::<_, Value>(sqlx::AssertSqlSafe(query.as_str()))
                    .bind(id)
                    .bind(owner)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(error)?
                    .ok_or(AccountError::NotFound)?,
            )
        })
    }
    fn create<'a>(&'a self, owner: &'a str, input: NewAccount) -> AccountFuture<'a, Account> {
        Box::pin(async move {
            input.validate()?;
            let mut tx = self.pool.begin().await.map_err(error)?;
            if let Some(wallet) = &input.wallet_id {
                let exists = sqlx::query_scalar::<_, String>(
                    r#"SELECT id FROM wallets WHERE id=$1 AND "userId"=$2 FOR UPDATE"#,
                )
                .bind(wallet)
                .bind(owner)
                .fetch_optional(&mut *tx)
                .await
                .map_err(error)?;
                if exists.is_none() {
                    return Err(AccountError::NotFound);
                }
            }
            let query = format!(
                r#"INSERT INTO "FinancialAccount" AS a (id,"userId",name,"institutionName",kind,"baseCurrency","walletId","updatedAt") VALUES ($1,$2,$3,$4,$5::"AccountKind",$6::"FiatCurrency",$7,timezone('UTC',now())) RETURNING {ACCOUNT}"#
            );
            let value = sqlx::query_scalar::<_, Value>(sqlx::AssertSqlSafe(query.as_str()))
                .bind(uuid::Uuid::new_v4().to_string())
                .bind(owner)
                .bind(input.name)
                .bind(input.institution_name)
                .bind(input.kind)
                .bind(input.base_currency)
                .bind(input.wallet_id)
                .fetch_one(&mut *tx)
                .await
                .map_err(error)?;
            let account = decode(value)?;
            tx.commit().await.map_err(error)?;
            Ok(account)
        })
    }
    fn remove<'a>(&'a self, owner: &'a str, id: &'a str) -> AccountFuture<'a, ()> {
        Box::pin(async move {
            let count =
                sqlx::query(r#"DELETE FROM "FinancialAccount" WHERE id=$1 AND "userId"=$2"#)
                    .bind(id)
                    .bind(owner)
                    .execute(&self.pool)
                    .await
                    .map_err(error)?
                    .rows_affected();
            if count == 0 {
                return Err(AccountError::NotFound);
            }
            Ok(())
        })
    }
    fn save_holding<'a>(
        &'a self,
        owner: &'a str,
        account: &'a str,
        id: Option<&'a str>,
        input: NewHolding,
    ) -> AccountFuture<'a, Holding> {
        Box::pin(async move {
            input.validate()?;
            let mut tx = self.pool.begin().await.map_err(error)?;
            if lock_account(&mut tx, owner, account).await? == "WALLET" {
                return Err(AccountError::Validation(
                    "Wallet positions come from wallet-assets",
                ));
            }
            let due = input
                .due_date
                .as_deref()
                .map(timestamp)
                .transpose()?
                .map(|v| v.naive_utc());
            let query = if id.is_some() {
                format!(
                    r#"UPDATE "Holding" AS h SET "assetClass"=$3::"AssetClass",symbol=$4,name=$5,quantity=$6::numeric,"unitPrice"=$7::numeric,"currentValue"=$8::numeric,"dueDate"=$9,"updatedAt"=timezone('UTC',now()) WHERE id=$1 AND "accountId"=$2 AND origin='MANUAL' RETURNING {HOLDING}"#
                )
            } else {
                format!(
                    r#"INSERT INTO "Holding" AS h (id,"accountId","assetClass",symbol,name,quantity,"unitPrice","currentValue","dueDate","updatedAt") VALUES ($1,$2,$3::"AssetClass",$4,$5,$6::numeric,$7::numeric,$8::numeric,$9,timezone('UTC',now())) RETURNING {HOLDING}"#
                )
            };
            let value = sqlx::query_scalar::<_, Value>(sqlx::AssertSqlSafe(query.as_str()))
                .bind(
                    id.map(str::to_owned)
                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                )
                .bind(account)
                .bind(input.asset_class)
                .bind(input.symbol)
                .bind(input.name)
                .bind(input.quantity)
                .bind(input.unit_price)
                .bind(input.current_value)
                .bind(due)
                .fetch_optional(&mut *tx)
                .await
                .map_err(error)?
                .ok_or(AccountError::NotFound)?;
            let holding = decode(value)?;
            tx.commit().await.map_err(error)?;
            Ok(holding)
        })
    }
    fn add_contribution<'a>(
        &'a self,
        owner: &'a str,
        account: &'a str,
        input: NewContribution,
    ) -> AccountFuture<'a, Contribution> {
        Box::pin(async move {
            input.validate()?;
            let mut tx = self.pool.begin().await.map_err(error)?;
            lock_account(&mut tx, owner, account).await?;
            let occurred = timestamp(&input.occurred_at)?.naive_utc();
            let query = format!(
                r#"INSERT INTO "Contribution" AS c (id,"accountId",kind,amount,currency,"fxRateToBrl","occurredAt",note) VALUES ($1,$2,$3::"ContributionKind",$4::numeric,$5::"FiatCurrency",$6::numeric,$7,$8) RETURNING {CONTRIBUTION}"#
            );
            let value = sqlx::query_scalar::<_, Value>(sqlx::AssertSqlSafe(query.as_str()))
                .bind(uuid::Uuid::new_v4().to_string())
                .bind(account)
                .bind(input.kind)
                .bind(input.amount)
                .bind(input.currency)
                .bind(input.fx_rate_to_brl)
                .bind(occurred)
                .bind(input.note)
                .fetch_one(&mut *tx)
                .await
                .map_err(error)?;
            let contribution = decode(value)?;
            tx.commit().await.map_err(error)?;
            Ok(contribution)
        })
    }
    fn remove_record<'a>(
        &'a self,
        owner: &'a str,
        account: &'a str,
        id: &'a str,
        holding: bool,
    ) -> AccountFuture<'a, ()> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(error)?;
            lock_account(&mut tx, owner, account).await?;
            let table = if holding { "Holding" } else { "Contribution" };
            let query = format!(
                r#"DELETE FROM "{table}" WHERE id=$1 AND "accountId"=$2 AND origin='MANUAL'"#
            );
            if sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
                .bind(id)
                .bind(account)
                .execute(&mut *tx)
                .await
                .map_err(error)?
                .rows_affected()
                == 0
            {
                return Err(AccountError::NotFound);
            }
            tx.commit().await.map_err(error)?;
            Ok(())
        })
    }
}
