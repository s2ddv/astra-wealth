use chrono::NaiveDateTime;
use domain::wallet::{
    Chain, NewWallet, Wallet, WalletAssetSummary, WalletFuture, WalletRepository,
    WalletRepositoryError,
};
use sqlx::PgPool;
use std::collections::HashMap;

/// Mirror the Prisma/Postgres enum at the adapter boundary, keeping SQLx out of domain.
#[derive(Clone, Copy, Debug, sqlx::Type)]
#[sqlx(type_name = "\"Chain\"", rename_all = "UPPERCASE")]
enum DbChain {
    Ethereum,
    Polygon,
    Arbitrum,
    Base,
    Solana,
    Bitcoin,
}
impl From<Chain> for DbChain {
    fn from(chain: Chain) -> Self {
        match chain {
            Chain::Ethereum => Self::Ethereum,
            Chain::Polygon => Self::Polygon,
            Chain::Arbitrum => Self::Arbitrum,
            Chain::Base => Self::Base,
            Chain::Solana => Self::Solana,
            Chain::Bitcoin => Self::Bitcoin,
        }
    }
}
impl From<DbChain> for Chain {
    fn from(chain: DbChain) -> Self {
        match chain {
            DbChain::Ethereum => Self::Ethereum,
            DbChain::Polygon => Self::Polygon,
            DbChain::Arbitrum => Self::Arbitrum,
            DbChain::Base => Self::Base,
            DbChain::Solana => Self::Solana,
            DbChain::Bitcoin => Self::Bitcoin,
        }
    }
}
#[derive(Clone)]
pub struct SqlxWalletRepository {
    pool: PgPool,
}
impl SqlxWalletRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
struct WalletRow {
    id: String,
    address: String,
    chain: DbChain,
    nickname: Option<String>,
    user_id: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    last_synced_at: Option<NaiveDateTime>,
}
impl From<WalletRow> for Wallet {
    fn from(row: WalletRow) -> Self {
        Self {
            id: row.id,
            address: row.address,
            chain: row.chain.into(),
            nickname: row.nickname,
            user_id: row.user_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            last_synced_at: row.last_synced_at,
            assets: Vec::new(),
        }
    }
}
struct AssetRow {
    id: String,
    symbol: String,
    amount: String,
    wallet_id: String,
    updated_at: NaiveDateTime,
}
impl From<AssetRow> for WalletAssetSummary {
    fn from(row: AssetRow) -> Self {
        Self {
            id: row.id,
            symbol: row.symbol,
            amount: decimal_text(&row.amount),
            wallet_id: row.wallet_id,
            updated_at: row.updated_at,
        }
    }
}
fn error(error: sqlx::Error) -> WalletRepositoryError {
    if error
        .as_database_error()
        .is_some_and(|error| error.is_unique_violation())
    {
        WalletRepositoryError::Conflict
    } else {
        WalletRepositoryError::Unavailable(Box::new(error))
    }
}

// Match Prisma Decimal.toString(): remove scale and use exponential notation
// below 1e-6, without passing NUMERIC(38,18) through a lossy float.
fn decimal_text(value: &str) -> String {
    let (sign, unsigned) = value.strip_prefix('-').map_or(("", value), |v| ("-", v));
    let (integer, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let integer = integer.trim_start_matches('0');
    let fraction = fraction.trim_end_matches('0');
    if integer.is_empty() && fraction.is_empty() {
        return "0".into();
    }
    if integer.is_empty() {
        let zeroes = fraction.bytes().take_while(|b| *b == b'0').count();
        if zeroes >= 6 {
            let digits = &fraction[zeroes..];
            let tail = if digits.len() > 1 {
                format!(".{}", &digits[1..])
            } else {
                String::new()
            };
            return format!("{sign}{}{tail}e-{}", &digits[..1], zeroes + 1);
        }
    }
    let integer = if integer.is_empty() { "0" } else { integer };
    if fraction.is_empty() {
        format!("{sign}{integer}")
    } else {
        format!("{sign}{integer}.{fraction}")
    }
}

impl WalletRepository for SqlxWalletRepository {
    fn create<'a>(&'a self, user_id: &'a str, wallet: NewWallet) -> WalletFuture<'a, Wallet> {
        Box::pin(async move {
            let id = uuid::Uuid::new_v4().to_string();
            let chain = DbChain::from(wallet.chain);
            // Owner comes from the authenticated service argument, never a JSON userId.
            sqlx::query_as!(WalletRow, r#"INSERT INTO wallets (id, address, chain, nickname, "userId", "createdAt", "updatedAt")
                VALUES ($1, $2, $3, $4, $5, timezone('UTC', now()), timezone('UTC', now()))
                RETURNING id, address, chain AS "chain: DbChain", nickname, "userId" AS user_id,
                "createdAt" AS created_at, "updatedAt" AS updated_at, "lastSyncedAt" AS last_synced_at"#,
                id, wallet.address, chain as DbChain, wallet.nickname, user_id)
                .fetch_one(&self.pool).await.map(Into::into).map_err(error)
        })
    }
    fn find_by_id<'a>(&'a self, id: &'a str, user_id: &'a str) -> WalletFuture<'a, Option<Wallet>> {
        Box::pin(async move {
            let row = sqlx::query_as!(WalletRow, r#"SELECT id, address, chain AS "chain: DbChain", nickname,
                "userId" AS user_id, "createdAt" AS created_at, "updatedAt" AS updated_at, "lastSyncedAt" AS last_synced_at
                FROM wallets WHERE id = $1 AND "userId" = $2"#, id, user_id)
                .fetch_optional(&self.pool).await.map_err(error)?;
            let Some(row) = row else { return Ok(None) };
            let mut wallet = Wallet::from(row);
            wallet.assets = sqlx::query_as!(
                AssetRow,
                r#"SELECT a.id, a.symbol, a.amount::text AS "amount!",
                a."walletId" AS wallet_id, a."updatedAt" AS updated_at FROM wallet_assets a
                JOIN wallets w ON w.id = a."walletId" WHERE w.id = $1 AND w."userId" = $2"#,
                id,
                user_id
            )
            .fetch_all(&self.pool)
            .await
            .map_err(error)?
            .into_iter()
            .map(Into::into)
            .collect();
            Ok(Some(wallet))
        })
    }
    fn find_by_user_id<'a>(&'a self, user_id: &'a str) -> WalletFuture<'a, Vec<Wallet>> {
        Box::pin(async move {
            let rows = sqlx::query_as!(WalletRow, r#"SELECT id, address, chain AS "chain: DbChain", nickname,
                "userId" AS user_id, "createdAt" AS created_at, "updatedAt" AS updated_at, "lastSyncedAt" AS last_synced_at
                FROM wallets WHERE "userId" = $1 ORDER BY "createdAt" DESC"#, user_id)
                .fetch_all(&self.pool).await.map_err(error)?;
            if rows.is_empty() {
                return Ok(Vec::new());
            }
            // One owner-scoped projection query avoids one query per wallet.
            let assets = sqlx::query_as!(
                AssetRow,
                r#"SELECT a.id, a.symbol, a.amount::text AS "amount!",
                a."walletId" AS wallet_id, a."updatedAt" AS updated_at FROM wallet_assets a
                JOIN wallets w ON w.id = a."walletId" WHERE w."userId" = $1"#,
                user_id
            )
            .fetch_all(&self.pool)
            .await
            .map_err(error)?;
            let mut by_wallet: HashMap<String, Vec<WalletAssetSummary>> = HashMap::new();
            for asset in assets {
                by_wallet
                    .entry(asset.wallet_id.clone())
                    .or_default()
                    .push(asset.into());
            }
            Ok(rows
                .into_iter()
                .map(|row| {
                    let mut wallet = Wallet::from(row);
                    wallet.assets = by_wallet.remove(&wallet.id).unwrap_or_default();
                    wallet
                })
                .collect())
        })
    }
    fn update_nickname<'a>(
        &'a self,
        id: &'a str,
        user_id: &'a str,
        nickname: &'a str,
    ) -> WalletFuture<'a, u64> {
        Box::pin(async move {
            sqlx::query!(
                r#"UPDATE wallets SET nickname = $3, "updatedAt" = timezone('UTC', now())
                WHERE id = $1 AND "userId" = $2"#,
                id,
                user_id,
                nickname
            )
            .execute(&self.pool)
            .await
            .map(|result| result.rows_affected())
            .map_err(error)
        })
    }
    fn delete<'a>(&'a self, id: &'a str, user_id: &'a str) -> WalletFuture<'a, u64> {
        Box::pin(async move {
            sqlx::query!(
                r#"DELETE FROM wallets WHERE id = $1 AND "userId" = $2"#,
                id,
                user_id
            )
            .execute(&self.pool)
            .await
            .map(|result| result.rows_affected())
            .map_err(error)
        })
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn prisma_decimal_strings_preserve_precision() {
        for (input, expected) in [
            ("0.000000000000000000", "0"),
            ("-0.000000000000000000", "0"),
            ("2.340000000000000000", "2.34"),
            ("0.000001000000000000", "0.000001"),
            ("0.000000123456789000", "1.23456789e-7"),
            ("-0.000000000000000001", "-1e-18"),
            (
                "99999999999999999999.123456789123456789",
                "99999999999999999999.123456789123456789",
            ),
        ] {
            assert_eq!(super::decimal_text(input), expected);
        }
    }
}
