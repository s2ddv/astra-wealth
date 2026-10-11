use application::operations::*;
use domain::accounts::{AccountError, AccountFuture, timestamp};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Row, Transaction};

pub struct SqlxOperationRepository {
    pool: PgPool,
}
impl SqlxOperationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
fn error(e: sqlx::Error) -> AccountError {
    if e.as_database_error()
        .is_some_and(|e| e.is_unique_violation() || e.is_foreign_key_violation())
    {
        AccountError::Conflict
    } else {
        AccountError::Unavailable(Box::new(e))
    }
}
async fn read(
    tx: &mut Transaction<'_, Postgres>,
    owner: &str,
    account: &str,
) -> Result<Vec<Operation>, AccountError> {
    let rows = sqlx::query_scalar::<_, Value>(
        r#"SELECT to_jsonb(o) || jsonb_build_object(
        'quantity',o.quantity::text,'amount',o.amount::text,'fees',o.fees::text,
        'occurredAt',to_char(o."occurredAt",'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"'),
        'createdAt',to_char(o."createdAt",'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"'))
        FROM financial_operations o JOIN "FinancialAccount" a ON a.id=o."accountId"
        WHERE a."userId"=$1 AND (o."accountId"=$2 OR o."destinationAccountId"=$2)
        ORDER BY o."occurredAt",o."createdAt",o.id"#,
    )
    .bind(owner)
    .bind(account)
    .fetch_all(&mut **tx)
    .await
    .map_err(error)?;
    rows.into_iter()
        .map(|v| serde_json::from_value(v).map_err(|e| AccountError::Unavailable(Box::new(e))))
        .collect()
}
fn normalized(mut v: NewOperation) -> Result<NewOperation, AccountError> {
    v.occurred_at = timestamp(&v.occurred_at)?.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    // Canonical decimal spelling for idempotent HTTP retries after storage round trips.
    for value in [&mut v.quantity, &mut v.amount, &mut v.fees]
        .into_iter()
        .flatten()
    {
        let (whole, fraction) = value.split_once('.').unwrap_or((value.as_str(), ""));
        let whole = whole.trim_start_matches('0');
        let whole = if whole.is_empty() { "0" } else { whole };
        let fraction = fraction.trim_end_matches('0');
        *value = if fraction.is_empty() {
            whole.to_string()
        } else {
            format!("{whole}.{fraction}")
        };
    }
    Ok(v)
}
impl OperationRepository for SqlxOperationRepository {
    fn list<'a>(&'a self, owner: &'a str, account: &'a str) -> AccountFuture<'a, Vec<Operation>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(error)?;
            let exists: bool = sqlx::query_scalar(
                r#"SELECT EXISTS(SELECT 1 FROM "FinancialAccount" WHERE id=$1 AND "userId"=$2)"#,
            )
            .bind(account)
            .bind(owner)
            .fetch_one(&mut *tx)
            .await
            .map_err(error)?;
            if !exists {
                return Err(AccountError::NotFound);
            }
            let records = read(&mut tx, owner, account).await?;
            tx.commit().await.map_err(error)?;
            Ok(records)
        })
    }
    fn append<'a>(
        &'a self,
        owner: &'a str,
        account: &'a str,
        input: NewOperation,
    ) -> AccountFuture<'a, Operation> {
        Box::pin(async move {
            input.validate()?;
            let input = normalized(input)?;
            let mut tx = self.pool.begin().await.map_err(error)?;
            // Serialize all ledger writes for this owner, including transfers and retries.
            sqlx::query_scalar::<_, String>("SELECT id FROM users WHERE id=$1 FOR UPDATE")
                .bind(owner)
                .fetch_optional(&mut *tx)
                .await
                .map_err(error)?
                .ok_or(AccountError::NotFound)?;
            let mut ids = vec![account];
            if let Some(id) = input.destination_account_id.as_deref() {
                if id == account {
                    return Err(AccountError::Validation(
                        "Origem e destino devem ser diferentes",
                    ));
                }
                ids.push(id);
            }
            ids.sort();
            for id in ids {
                let a = sqlx::query(r#"SELECT kind::text,"baseCurrency"::text AS currency FROM "FinancialAccount" WHERE id=$1 AND "userId"=$2 FOR UPDATE"#).bind(id).bind(owner).fetch_optional(&mut *tx).await.map_err(error)?.ok_or(AccountError::NotFound)?;
                let kind: String = a.try_get("kind").map_err(error)?;
                if !["MANUAL", "BANK", "BROKERAGE"].contains(&kind.as_str()) {
                    return Err(AccountError::Validation(
                        "Operações manuais exigem conta manual, bancária ou corretora",
                    ));
                }
                if a.try_get::<String, _>("currency").map_err(error)? != input.currency {
                    return Err(AccountError::Validation(
                        "Moeda deve coincidir com as contas; transferência com câmbio não suportada",
                    ));
                }
            }
            let records = read(&mut tx, owner, account).await?;
            if let Some(old) = records
                .iter()
                .find(|o| o.account_id == account && o.input.request_id == input.request_id)
            {
                if normalized(old.input.clone())? != input {
                    return Err(AccountError::Conflict);
                }
                return Ok(old.clone());
            }
            let asset_class = if let Some(holding) = &input.holding_id {
                Some(sqlx::query_scalar::<_,String>(r#"SELECT "assetClass"::text FROM "Holding" WHERE id=$1 AND "accountId"=$2 AND origin='MANUAL' FOR SHARE"#).bind(holding).bind(account).fetch_optional(&mut *tx).await.map_err(error)?.ok_or(AccountError::NotFound)?)
            } else {
                None
            };
            if let Some(id) = &input.replaces_id {
                let old = records
                    .iter()
                    .find(|o| &o.id == id && o.account_id == account)
                    .ok_or(AccountError::NotFound)?;
                if records
                    .iter()
                    .any(|o| o.input.replaces_id.as_ref() == Some(id))
                {
                    return Err(AccountError::Conflict);
                }
                // Immutable references keep both sides of transfer histories auditable.
                if old.input.kind != input.kind
                    || old.input.holding_id != input.holding_id
                    || old.input.destination_account_id != input.destination_account_id
                    || old.input.currency != input.currency
                    || old.input.market != input.market
                    || old.asset_class != asset_class
                {
                    return Err(AccountError::Validation(
                        "Correção preserva tipo, ativo, contas, mercado e moeda",
                    ));
                }
            }
            let count: i64 = sqlx::query_scalar(r#"SELECT count(*) FROM financial_operations o JOIN "FinancialAccount" a ON a.id=o."accountId" WHERE a."userId"=$1"#).bind(owner).fetch_one(&mut *tx).await.map_err(error)?;
            if count >= 2000 {
                return Err(AccountError::Validation(
                    "Limite desta etapa: 2000 registros por usuário, incluindo correções",
                ));
            }
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query(r#"INSERT INTO financial_operations(id,"accountId","destinationAccountId","holdingId","replacesId","requestId",kind,"assetClass",market,quantity,amount,fees,currency,"occurredAt",note)
                VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10::numeric,$11::numeric,$12::numeric,$13::"FiatCurrency",$14,$15)"#)
                .bind(&id).bind(account).bind(&input.destination_account_id).bind(&input.holding_id).bind(&input.replaces_id).bind(&input.request_id).bind(&input.kind).bind(asset_class).bind(&input.market).bind(&input.quantity).bind(&input.amount).bind(&input.fees).bind(&input.currency).bind(timestamp(&input.occurred_at)?.naive_utc()).bind(&input.note).execute(&mut *tx).await.map_err(error)?;
            let records = read(&mut tx, owner, account).await?;
            let ledger = replay(records, account)?; // Invalid historical changes roll back atomically.
            let saved = ledger
                .operations
                .into_iter()
                .find(|o| o.id == id)
                .ok_or(AccountError::NotFound)?;
            tx.commit().await.map_err(error)?;
            Ok(saved)
        })
    }
}
