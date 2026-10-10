use super::*;
use crate::fx::{FxFuture, FxProvider, FxQuote};
use domain::asset_icon::{AssetCache, IconFuture};
struct Cache;
impl AssetCache for Cache {
    fn get<'a>(&'a self, _: &'a str) -> IconFuture<'a, Option<String>> {
        Box::pin(async { Ok(None) })
    }
    fn set<'a>(&'a self, _: &'a str, _: &'a str, _: u64) -> IconFuture<'a, ()> {
        Box::pin(async { Ok(()) })
    }
}
struct Fx;
impl FxProvider for Fx {
    fn closing_on_or_before(&self, date: NaiveDate) -> FxFuture<'_> {
        Box::pin(async move {
            Ok(Some(FxQuote {
                rate: if date.to_string() == "2020-01-02" {
                    "4"
                } else {
                    "5"
                }
                .into(),
                effective_date: date.to_string(),
                published_at: format!("{date} 13:00:00"),
            }))
        })
    }
}
struct Prices;
impl BrlPriceProvider for Prices {
    fn price<'a>(
        &'a self,
        _: Chain,
        contract: Option<&'a str>,
    ) -> PortfolioFuture<'a, Option<BrlQuote>> {
        Box::pin(async move {
            Ok(contract.is_none().then(|| BrlQuote {
                price: "10000.123456789".into(),
                fetched_at: "2026-04-06T18:00:00Z".into(),
            }))
        })
    }
}
struct Repo;
impl PortfolioRepository for Repo {
    fn load<'a>(&'a self, _: &'a str) -> PortfolioFuture<'a, PortfolioInput> {
        Box::pin(async { Err(PortfolioError) })
    }
    fn save<'a>(&'a self, _: &'a str, _: &'a PortfolioSnapshot) -> PortfolioFuture<'a, String> {
        Box::pin(async { Err(PortfolioError) })
    }
    fn history<'a>(&'a self, _: &'a str) -> PortfolioFuture<'a, Vec<SavedSnapshot>> {
        Box::pin(async { Err(PortfolioError) })
    }
}
fn service() -> PortfolioService {
    PortfolioService::new(
        Arc::new(Repo),
        FxService::new(Arc::new(Fx), Arc::new(Cache)),
        Arc::new(Prices),
    )
}
fn position(id: &str, kind: PositionKind) -> PositionInput {
    PositionInput {
        id: id.into(),
        account_id: None,
        wallet_id: None,
        name: id.into(),
        updated_at: "2026-04-06T18:00:00Z".into(),
        issue: None,
        kind,
    }
}
fn contribution(id: &str, recorded_rate: Option<&str>, withdrawal: bool) -> ContributionInput {
    ContributionInput {
        id: id.into(),
        amount: "10".into(),
        currency: "USD".into(),
        withdrawal,
        date: "2020-01-02".parse().unwrap(),
        recorded_rate: recorded_rate.map(Into::into),
    }
}
#[tokio::test]
async fn exact_current_values_historical_contributions_and_no_inferred_deposits() {
    let input = PortfolioInput {
        observed_at: "2026-04-06T18:00:00Z".into(),
        positions: vec![
            position(
                "cash",
                PositionKind::Fiat {
                    currency: "USD".into(),
                    value: "100.1234567891".into(),
                },
            ),
            position(
                "eth",
                PositionKind::Crypto {
                    chain: Chain::Ethereum,
                    contract: None,
                    amount: "0.000000000000000001".into(),
                },
            ),
        ],
        contributions: vec![
            contribution("manual", Some("3.12345678"), false),
            contribution("historical", None, false),
            contribution("withdrawal", Some("2"), true),
        ],
    };
    let s = service()
        .consolidate(input, "2026-04-06T18:00:00Z".parse().unwrap())
        .await
        .unwrap();
    assert_eq!(
        s.known_value_brl.as_deref(),
        Some("500.617283945500010000123456789")
    );
    assert_eq!(s.net_contributions_brl.as_deref(), Some("51.2345678"));
    assert!(s.complete);
    assert_eq!(s.contributions.len(), 3);
    assert_eq!(s.contributions[0].source, "RECORDED");
    assert_eq!(s.contributions[0].rate.as_deref(), Some("3.12345678"));
    assert_eq!(
        s.contributions[1]
            .fx
            .as_ref()
            .unwrap()
            .quote
            .as_ref()
            .unwrap()
            .effective_date,
        "2020-01-02"
    );
    assert_eq!(s.positions[1].source, "COINGECKO_BRL");
    assert!(s.positions[1].fx.is_none());
}
#[tokio::test]
async fn partial_missing_empty_and_unsynced_are_distinct() {
    let now = "2026-04-06T18:00:00Z".parse().unwrap();
    let missing = position(
        "unknown",
        PositionKind::Crypto {
            chain: Chain::Ethereum,
            contract: Some("unknown".into()),
            amount: "1".into(),
        },
    );
    let input = |positions| PortfolioInput {
        observed_at: "now".into(),
        positions,
        contributions: vec![],
    };
    let s = service()
        .consolidate(input(vec![missing.clone()]), now)
        .await
        .unwrap();
    assert_eq!(s.known_value_brl, None);
    assert!(!s.complete);
    assert_eq!(s.result_brl, None);
    assert_eq!(s.net_contributions_brl.as_deref(), Some("0"));
    let s = service()
        .consolidate(
            input(vec![
                missing,
                position(
                    "cash",
                    PositionKind::Fiat {
                        currency: "BRL".into(),
                        value: "0.1".into(),
                    },
                ),
            ]),
            now,
        )
        .await
        .unwrap();
    assert_eq!(s.known_value_brl.as_deref(), Some("0.1"));
    assert!(!s.complete);
    let s = service().consolidate(input(vec![]), now).await.unwrap();
    assert!(s.complete);
    assert_eq!(s.known_value_brl.as_deref(), Some("0"));
    let mut p = position("wallet", PositionKind::EmptyWallet);
    p.issue = Some("NOT_SYNCED".into());
    let s = service().consolidate(input(vec![p]), now).await.unwrap();
    assert!(!s.complete);
    assert_eq!(s.known_value_brl, None);
}
