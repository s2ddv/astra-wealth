use super::*;
#[test]
fn exact_json_decimals_and_closing_validation() {
    let d = "2026-04-06".parse().unwrap();
    let payload = r#"{"value":[{"cotacaoVenda":5.123456789012345678,"dataHoraCotacao":"2026-04-06 13:05:12.123","tipoBoletim":"Fechamento"}]}"#;
    assert_eq!(
        parse(payload.as_bytes(), d).unwrap().unwrap().rate,
        "5.123456789012345678"
    );
    for body in [
        payload.replace("Fechamento", "Intermediário"),
        payload.replace("5.123456789012345678", "0"),
        payload.replace("2026-04-06", "2026-04-07"),
        "{}".into(),
    ] {
        assert!(parse(body.as_bytes(), d).is_err());
    }
    assert!(parse(br#"{"value":[]}"#, d).unwrap().is_none());
}
#[tokio::test]
async fn http_contract_and_outage() {
    use axum::{Router, extract::RawQuery, routing::get};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = Router::new().fallback(get(|RawQuery(query): RawQuery| async move {
        assert!(!query.as_ref().unwrap().contains('+'));
        let query: std::collections::HashMap<_, _> = url::form_urlencoded::parse(query.unwrap().as_bytes()).into_owned().collect();
        assert_eq!(query["@moeda"], "'USD'");
        assert_eq!(query["@dataFinalCotacao"], "'04-06-2026'");
        assert_eq!(query["$filter"], "tipoBoletim eq 'Fechamento'");
        assert_eq!(query["$orderby"], "dataHoraCotacao desc");
        assert_eq!(query["$top"], "1");
        r#"{"value":[{"cotacaoVenda":5.1234,"dataHoraCotacao":"2026-04-02 13:05:12.123","tipoBoletim":"Fechamento"}]}"#
    }));
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let mut provider = BcbPtaxProvider::new().unwrap();
    provider.base = format!("http://{addr}");
    let d = "2026-04-06".parse().unwrap();
    assert_eq!(
        provider
            .closing_on_or_before(d)
            .await
            .unwrap()
            .unwrap()
            .rate,
        "5.1234"
    );
    task.abort();
    let _ = task.await;
    assert!(provider.closing_on_or_before(d).await.is_err());
}
#[tokio::test]
#[ignore = "Requires network access to the Banco Central public API"]
async fn live_historical_ptax() {
    let quote = BcbPtaxProvider::new()
        .unwrap()
        .closing_on_or_before("2024-01-07".parse().unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(quote.effective_date, "2024-01-05");
    assert!(application::tracking::price_decimal(&quote.rate).unwrap() > 0);
}

#[tokio::test]
#[ignore = "Requires network access to the Banco Central public API"]
async fn live_current_ptax() {
    let date = application::fx::brasilia_today(chrono::Utc::now());
    let quote = BcbPtaxProvider::new()
        .unwrap()
        .closing_on_or_before(date)
        .await
        .unwrap()
        .unwrap();
    assert!(application::fx::valid_quote(&quote, date));
    println!(
        "Current PTAX: requested={date}, effective={}, rate={}",
        quote.effective_date, quote.rate
    );
}
