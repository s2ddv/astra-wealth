use super::*;
fn op(id: &str, kind: &str, q: &str, a: Option<&str>, fees: Option<&str>) -> Operation {
    Operation {
        id: id.into(),
        account_id: "account".into(),
        asset_class: Some("STOCK".into()),
        created_at: format!("2020-01-0{id}T00:00:00.000Z"),
        input: NewOperation {
            request_id: id.into(),
            kind: kind.into(),
            holding_id: Some("holding".into()),
            destination_account_id: None,
            replaces_id: None,
            market: Some("BR".into()),
            quantity: Some(q.into()),
            amount: a.map(str::to_owned),
            fees: fees.map(str::to_owned),
            currency: "BRL".into(),
            occurred_at: format!("2020-01-0{id}T00:00:00Z"),
            note: None,
        },
    }
}
#[test]
fn weighted_cost_fees_partial_and_full_sales() {
    let l = replay(
        vec![
            op("1", "BUY", "10", Some("100"), Some("2")),
            op("2", "BUY", "10", Some("200"), Some("4")),
            op("3", "SELL", "5", Some("100"), Some("1")),
            op("4", "SELL", "15", Some("300"), Some("3")),
        ],
        "account",
    )
    .unwrap();
    assert_eq!(l.results[2].assigned_cost.as_deref(), Some("76.5"));
    assert_eq!(l.results[2].realized_result.as_deref(), Some("22.5"));
    assert_eq!(l.results[3].assigned_cost.as_deref(), Some("229.5"));
    assert_eq!(l.results[3].realized_result.as_deref(), Some("67.5"));
    assert_eq!(l.positions[0].quantity, "0");
    assert_eq!(l.positions[0].accumulated_cost.as_deref(), Some("0"));
}
#[test]
fn recurring_average_consumes_exact_remaining_cost_on_final_sale() {
    let l = replay(
        vec![
            op("1", "BUY", "3", Some("1"), Some("0")),
            op("2", "SELL", "1", Some("1"), Some("0")),
            op("3", "SELL", "2", Some("1"), Some("0")),
        ],
        "account",
    )
    .unwrap();
    assert_eq!(
        l.results[1].assigned_cost.as_deref(),
        Some("0.33333333333333333333")
    );
    assert_eq!(
        l.results[2].assigned_cost.as_deref(),
        Some("0.66666666666666666667")
    );
}
#[test]
fn unknown_inputs_and_unsupported_classes_never_fabricate_results() {
    for (class, market, currency) in [
        ("CRYPTO", "BR", "BRL"),
        ("FIXED_INCOME", "BR", "BRL"),
        ("STOCK", "INTERNATIONAL", "BRL"),
        ("ETF", "UNKNOWN", "BRL"),
        ("STOCK", "BR", "USD"),
    ] {
        let mut ops = vec![
            op("1", "BUY", "1", Some("10"), Some("0")),
            op("2", "SELL", "1", Some("20"), Some("0")),
        ];
        for o in &mut ops {
            o.asset_class = Some(class.into());
            o.input.market = Some(market.into());
            o.input.currency = currency.into();
        }
        let l = replay(ops, "account").unwrap();
        assert!(l.results[1].realized_result.is_none());
        assert_eq!(
            l.results[1].unavailable_reason.as_deref(),
            Some("UNSUPPORTED_COST_RULE")
        );
    }
    for (amount, fees) in [(None, Some("0")), (Some("10"), None)] {
        let l = replay(
            vec![
                op("1", "BUY", "1", amount, fees),
                op("2", "SELL", "1", Some("20"), Some("0")),
            ],
            "account",
        )
        .unwrap();
        assert!(l.results[1].realized_result.is_none());
    }
    let l = replay(
        vec![
            op("1", "OPENING", "2", None, None),
            op("2", "SELL", "1", Some("20"), Some("0")),
        ],
        "account",
    )
    .unwrap();
    assert!(l.positions[0].accumulated_cost.is_none());
    assert!(l.results[1].realized_result.is_none());
}
#[test]
fn corrections_replay_original_order_and_preserve_originals() {
    let a = op("1", "BUY", "2", Some("10"), Some("0"));
    let b = op("2", "SELL", "1", Some("20"), Some("0"));
    let mut correction = op("3", "BUY", "2", Some("12"), Some("0"));
    correction.input.occurred_at = a.input.occurred_at.clone();
    correction.input.replaces_id = Some(a.id.clone());
    let l = replay(vec![b, a, correction], "account").unwrap();
    assert_eq!(l.operations.len(), 3);
    assert!(!l.results[0].active);
    assert_eq!(l.results[2].realized_result.as_deref(), Some("14"));
    assert!(replay(vec![op("1", "SELL", "1", Some("20"), Some("0"))], "account").is_err());
    assert!(
        replay(
            vec![
                op("1", "BUY", "1", Some("10"), Some("0")),
                op("2", "OPENING", "1", None, None)
            ],
            "account"
        )
        .is_err()
    );
}
#[test]
fn cash_events_do_not_modify_cost_or_quantities() {
    let buy = op("1", "BUY", "1", Some("10"), Some("0"));
    let mut fee = op("2", "FEE", "1", Some("2"), None);
    fee.input.quantity = None;
    fee.input.holding_id = None;
    fee.input.market = None;
    fee.asset_class = None;
    let mut transfer = fee.clone();
    transfer.id = "3".into();
    transfer.input.kind = "TRANSFER".into();
    transfer.input.destination_account_id = Some("destination".into());
    let l = replay(vec![buy, fee, transfer.clone()], "account").unwrap();
    assert_eq!(l.positions[0].accumulated_cost.as_deref(), Some("10"));
    assert!(l.results[1].realized_result.is_none());
    assert!(
        replay(vec![transfer], "destination")
            .unwrap()
            .positions
            .is_empty()
    );
}
#[test]
fn rejects_invalid_payloads_and_preserves_subcent_precision() {
    let good = op(
        "1",
        "BUY",
        "0.0000000001",
        Some("999999999999999999.1234567891"),
        Some("0"),
    );
    good.input.validate().unwrap();
    let l = replay(vec![good.clone()], "account").unwrap();
    assert_eq!(
        l.positions[0].accumulated_cost.as_deref(),
        Some("999999999999999999.1234567891")
    );
    for q in ["NaN", "1e3", "-1", "0", "0.00000000001"] {
        let mut bad = good.input.clone();
        bad.quantity = Some(q.into());
        assert!(bad.validate().is_err());
    }
    let mut bad = good.input;
    bad.kind = "TRANSFER".into();
    assert!(bad.validate().is_err());
    let mut json = serde_json::to_value(bad).unwrap();
    json["userId"] = serde_json::json!("foreign");
    assert!(serde_json::from_value::<NewOperation>(json).is_err());
}
