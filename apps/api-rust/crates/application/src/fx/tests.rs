use super::*;
use domain::asset_icon::IconFuture;
use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
#[derive(Default)]
struct Cache(Mutex<HashMap<String, String>>);
impl AssetCache for Cache {
    fn get<'a>(&'a self, key: &'a str) -> IconFuture<'a, Option<String>> {
        Box::pin(async move { Ok(self.0.lock().unwrap().get(key).cloned()) })
    }
    fn set<'a>(&'a self, key: &'a str, value: &'a str, _: u64) -> IconFuture<'a, ()> {
        Box::pin(async move {
            self.0.lock().unwrap().insert(key.into(), value.into());
            Ok(())
        })
    }
}
struct Provider {
    quote: Mutex<Option<FxQuote>>,
    fail: Mutex<bool>,
    calls: AtomicUsize,
}
impl FxProvider for Provider {
    fn closing_on_or_before(&self, _: NaiveDate) -> FxFuture<'_> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if *self.fail.lock().unwrap() {
                Err(FxError::Unavailable)
            } else {
                Ok(self.quote.lock().unwrap().clone())
            }
        })
    }
}
fn quote(date: &str) -> FxQuote {
    FxQuote {
        rate: "5.123456789012345678".into(),
        effective_date: date.into(),
        published_at: format!("{date} 13:05:12.123"),
    }
}
fn date(value: &str) -> NaiveDate {
    value.parse().unwrap()
}
fn now(value: &str) -> DateTime<Utc> {
    value.parse().unwrap()
}
fn setup(q: Option<FxQuote>) -> (FxService, Arc<Provider>) {
    let provider = Arc::new(Provider {
        quote: Mutex::new(q),
        fail: Mutex::new(false),
        calls: AtomicUsize::new(0),
    });
    (
        FxService::new(provider.clone(), Arc::new(Cache::default())),
        provider,
    )
}
#[tokio::test]
async fn weekends_holidays_and_unpublished_close_use_previous_date() {
    for day in ["2026-04-03", "2026-04-04", "2026-04-05", "2026-04-06"] {
        let (service, _) = setup(Some(quote("2026-04-02")));
        let result = service
            .rate(date(day), now("2026-04-06T15:00:00Z"))
            .await
            .unwrap();
        assert_eq!(result.status, "previous_close");
        assert_eq!(result.quote.unwrap().effective_date, "2026-04-02");
    }
}
#[tokio::test]
async fn current_cache_refreshes_after_publication_and_expires_stale() {
    let (service, provider) = setup(Some(quote("2026-04-02")));
    let d = date("2026-04-06");
    let t = now("2026-04-06T15:00:00Z");
    service.rate(d, t).await.unwrap();
    service.rate(d, t + Duration::seconds(299)).await.unwrap();
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    *provider.quote.lock().unwrap() = Some(quote("2026-04-06"));
    let fresh = service.rate(d, t + Duration::minutes(6)).await.unwrap();
    assert_eq!(fresh.status, "available");
    assert_eq!(fresh.quote.unwrap().rate, "5.123456789012345678");
    *provider.fail.lock().unwrap() = true;
    let stale = service.rate(d, t + Duration::minutes(12)).await.unwrap();
    assert_eq!(stale.status, "stale");
    assert_eq!(stale.fetched_at, (t + Duration::minutes(6)).to_rfc3339());
    assert_eq!(
        service.rate(d, t + Duration::days(2)).await.unwrap().status,
        "unavailable"
    );
}
#[tokio::test]
async fn historical_date_never_uses_today_quote_and_negative_cache_is_short() {
    let (service, provider) = setup(None);
    let d = date("2020-02-02");
    let t = now("2026-04-06T15:00:00Z");
    assert_eq!(service.rate(d, t).await.unwrap().status, "missing");
    *provider.quote.lock().unwrap() = Some(quote("2020-01-31"));
    assert_eq!(
        service
            .rate(d, t + Duration::seconds(61))
            .await
            .unwrap()
            .status,
        "previous_close"
    );
    let (service, _) = setup(Some(quote("2026-04-06")));
    assert_eq!(service.rate(d, t).await.unwrap().status, "unavailable");
    assert!(service.rate(date("2026-04-07"), t).await.is_err());
    assert_eq!(
        brasilia_today(now("2026-04-07T02:59:00Z")),
        date("2026-04-06")
    );
}
