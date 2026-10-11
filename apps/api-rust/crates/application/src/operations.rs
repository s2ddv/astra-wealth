//! Manual bookkeeping, independent of manually supplied market valuations.
use bigdecimal::{BigDecimal, RoundingMode};
use domain::accounts::{AccountError, AccountFuture, decimal, timestamp};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    str::FromStr,
    sync::Arc,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewOperation {
    pub request_id: String,
    pub kind: String,
    pub holding_id: Option<String>,
    pub destination_account_id: Option<String>,
    pub replaces_id: Option<String>,
    pub market: Option<String>,
    pub quantity: Option<String>,
    /// Gross consideration; OPENING uses total acquisition cost, when known.
    pub amount: Option<String>,
    /// None means unknown, never zero. Included only in BUY/SELL/INCOME.
    pub fees: Option<String>,
    pub currency: String,
    pub occurred_at: String,
    pub note: Option<String>,
}
impl NewOperation {
    pub fn validate(&self) -> Result<(), AccountError> {
        let fail = || AccountError::Validation("Campos incompatíveis com o tipo de operação");
        for id in [
            Some(&self.request_id),
            self.holding_id.as_ref(),
            self.destination_account_id.as_ref(),
            self.replaces_id.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if id.is_empty()
                || id.len() > 200
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-:".contains(&b))
            {
                return Err(fail());
            }
        }
        if !["BRL", "USD"].contains(&self.currency.as_str())
            || timestamp(&self.occurred_at)? > chrono::Utc::now()
            || self.note.as_ref().is_some_and(|s| s.chars().count() > 1000)
        {
            return Err(fail());
        }
        for v in [&self.amount, &self.fees].into_iter().flatten() {
            decimal(v, 18, 10, false)?;
        }
        if let Some(q) = &self.quantity {
            decimal(q, 18, 10, true)?;
        }
        let trade = ["BUY", "SELL", "OPENING"].contains(&self.kind.as_str());
        if trade {
            if self.holding_id.is_none()
                || self.quantity.is_none()
                || !self
                    .market
                    .as_deref()
                    .is_some_and(|m| ["BR", "INTERNATIONAL", "UNKNOWN"].contains(&m))
                || self.destination_account_id.is_some()
            {
                return Err(fail());
            }
            if self.kind == "OPENING" && self.fees.is_some() {
                return Err(fail());
            }
        } else {
            if self.quantity.is_some() || self.market.is_some() || self.amount.is_none() {
                return Err(fail());
            }
            match self.kind.as_str() {
                "INCOME" if self.destination_account_id.is_none() => {}
                "FEE"
                    if self.holding_id.is_none()
                        && self.destination_account_id.is_none()
                        && self.fees.is_none() => {}
                "TRANSFER"
                    if self.holding_id.is_none()
                        && self.destination_account_id.is_some()
                        && self.fees.is_none() => {}
                _ => return Err(fail()),
            }
            decimal(self.amount.as_deref().unwrap(), 18, 10, true)?;
        }
        if ["SELL", "INCOME"].contains(&self.kind.as_str())
            && let (Some(a), Some(f)) = (&self.amount, &self.fees)
            && number(f)? > number(a)?
        {
            return Err(AccountError::Validation(
                "Taxas excedem o valor bruto recebido",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub id: String,
    pub account_id: String,
    pub asset_class: Option<String>,
    pub created_at: String,
    #[serde(flatten)]
    pub input: NewOperation,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationResult {
    pub id: String,
    pub active: bool,
    pub net_amount: Option<String>,
    pub assigned_cost: Option<String>,
    pub realized_result: Option<String>,
    pub unavailable_reason: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerPosition {
    pub holding_id: String,
    pub quantity: String,
    pub accumulated_cost: Option<String>,
    pub average_cost: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationLedger {
    pub operations: Vec<Operation>,
    pub results: Vec<OperationResult>,
    pub positions: Vec<LedgerPosition>,
}
pub trait OperationRepository: Send + Sync {
    fn list<'a>(&'a self, owner: &'a str, account: &'a str) -> AccountFuture<'a, Vec<Operation>>;
    /// Atomically locks owner, authorizes all references, checks replay and appends.
    fn append<'a>(
        &'a self,
        owner: &'a str,
        account: &'a str,
        input: NewOperation,
    ) -> AccountFuture<'a, Operation>;
}
#[derive(Clone)]
pub struct OperationService {
    repository: Arc<dyn OperationRepository>,
}
impl OperationService {
    pub fn new(repository: Arc<dyn OperationRepository>) -> Self {
        Self { repository }
    }
    pub async fn list(&self, owner: &str, account: &str) -> Result<OperationLedger, AccountError> {
        replay(self.repository.list(owner, account).await?, account)
    }
    pub async fn append(
        &self,
        owner: &str,
        account: &str,
        input: NewOperation,
    ) -> Result<Operation, AccountError> {
        input.validate()?;
        self.repository.append(owner, account, input).await
    }
}
fn number(v: &str) -> Result<BigDecimal, AccountError> {
    BigDecimal::from_str(v).map_err(|_| AccountError::Validation("Decimal inválido"))
}
fn optional(v: &Option<String>) -> Result<Option<BigDecimal>, AccountError> {
    v.as_deref().map(number).transpose()
}
fn plain(v: BigDecimal) -> String {
    v.normalized().to_plain_string()
}
fn ratio(cost: &BigDecimal, sold: &BigDecimal, quantity: &BigDecimal) -> BigDecimal {
    // Explicit decimal rounding only at a recurring division, never binary floats.
    (cost * sold / quantity).with_scale_round(20, RoundingMode::HalfEven)
}
#[derive(Default)]
struct Position {
    quantity: BigDecimal,
    cost: Option<BigDecimal>,
    classification: Option<(String, String, String)>,
    seen: bool,
}
/// Full chronological replay. Corrections are append-only; inactive originals remain visible.
pub fn replay(
    mut operations: Vec<Operation>,
    account: &str,
) -> Result<OperationLedger, AccountError> {
    operations.sort_by(|a, b| {
        (timestamp(&a.input.occurred_at).ok(), &a.created_at, &a.id).cmp(&(
            timestamp(&b.input.occurred_at).ok(),
            &b.created_at,
            &b.id,
        ))
    });
    let replaced: HashSet<String> = operations
        .iter()
        .filter_map(|o| o.input.replaces_id.clone())
        .collect();
    let mut positions: BTreeMap<String, Position> = BTreeMap::new();
    let mut results = Vec::new();
    for op in &operations {
        let v = &op.input;
        let active = !replaced.contains(&op.id);
        let mut r = OperationResult {
            id: op.id.clone(),
            active,
            net_amount: None,
            assigned_cost: None,
            realized_result: None,
            unavailable_reason: None,
        };
        if !active {
            results.push(r);
            continue;
        }
        let amount = optional(&v.amount)?;
        let fees = optional(&v.fees)?;
        if ["BUY", "SELL", "INCOME"].contains(&v.kind.as_str()) {
            r.net_amount = amount
                .as_ref()
                .zip(fees.as_ref())
                .map(|(a, f)| plain(if v.kind == "BUY" { a + f } else { a - f }));
        } else {
            r.net_amount = v.amount.clone();
        }
        if op.account_id == account && ["BUY", "SELL", "OPENING"].contains(&v.kind.as_str()) {
            let p = positions
                .entry(
                    v.holding_id
                        .clone()
                        .ok_or(AccountError::Validation("Ativo ausente"))?,
                )
                .or_default();
            let classification = (
                op.asset_class.clone().unwrap_or_default(),
                v.market.clone().unwrap_or_default(),
                v.currency.clone(),
            );
            if p.classification
                .as_ref()
                .is_some_and(|c| c != &classification)
            {
                return Err(AccountError::Validation(
                    "Classe, mercado e moeda devem ser consistentes no histórico do ativo",
                ));
            }
            p.classification = Some(classification);
            let supported = v.market.as_deref() == Some("BR")
                && v.currency == "BRL"
                && op
                    .asset_class
                    .as_deref()
                    .is_some_and(|c| ["STOCK", "REAL_ESTATE_FUND", "ETF"].contains(&c));
            if !p.seen {
                p.cost = supported.then(BigDecimal::default);
            }
            let q = number(
                v.quantity
                    .as_deref()
                    .ok_or(AccountError::Validation("Quantidade ausente"))?,
            )?;
            match v.kind.as_str() {
                "OPENING" => {
                    if p.seen {
                        return Err(AccountError::Validation(
                            "Saldo inicial deve ser o primeiro registro do ativo",
                        ));
                    }
                    p.quantity = q;
                    p.cost = if supported { amount } else { None };
                }
                "BUY" => {
                    p.quantity += q;
                    p.cost = p
                        .cost
                        .as_ref()
                        .zip(amount.as_ref())
                        .zip(fees.as_ref())
                        .map(|((c, a), f)| c + a + f);
                }
                "SELL" => {
                    if q > p.quantity {
                        return Err(AccountError::Validation(
                            "Venda excede a quantidade do histórico; registre o saldo inicial ou corrija as operações",
                        ));
                    }
                    let assigned = p.cost.as_ref().map(|c| {
                        if q == p.quantity {
                            c.clone()
                        } else {
                            ratio(c, &q, &p.quantity)
                        }
                    });
                    r.assigned_cost = assigned.clone().map(plain);
                    r.realized_result = assigned
                        .as_ref()
                        .zip(amount.as_ref())
                        .zip(fees.as_ref())
                        .map(|((c, a), f)| plain(a - f - c));
                    p.cost = p.cost.as_ref().zip(assigned.as_ref()).map(|(c, s)| c - s);
                    p.quantity -= q;
                    if p.quantity == BigDecimal::default() {
                        p.cost = supported.then(BigDecimal::default);
                    }
                }
                _ => unreachable!(),
            }
            p.seen = true;
            if !supported {
                r.unavailable_reason = Some("UNSUPPORTED_COST_RULE".into());
            } else if v.kind == "SELL" && r.realized_result.is_none() {
                r.unavailable_reason = Some("MISSING_COST_AMOUNT_OR_FEES".into());
            }
        }
        results.push(r);
    }
    let positions = positions
        .into_iter()
        .map(|(id, p)| LedgerPosition {
            holding_id: id,
            average_cost: p.cost.as_ref().and_then(|c| {
                (p.quantity != BigDecimal::default())
                    .then(|| plain(ratio(c, &BigDecimal::from(1), &p.quantity)))
            }),
            quantity: plain(p.quantity),
            accumulated_cost: p.cost.map(plain),
        })
        .collect();
    Ok(OperationLedger {
        operations,
        results,
        positions,
    })
}
#[cfg(test)]
mod tests;
