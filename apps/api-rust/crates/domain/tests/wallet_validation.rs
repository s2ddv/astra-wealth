use zora_domain::wallet::{Chain, NewWallet, NicknameUpdate};

#[test]
fn validates_each_chain_and_evm_checksum() {
    for (chain, address) in [
        (
            Chain::Ethereum,
            "0x52908400098527886E0F7030069857D2E4169EE7",
        ),
        (Chain::Polygon, "0xde709f2102306220921060314715629080e2fb77"),
        (
            Chain::Arbitrum,
            "0xde709f2102306220921060314715629080e2fb77",
        ),
        (Chain::Base, "0xde709f2102306220921060314715629080e2fb77"),
        (Chain::Solana, "11111111111111111111111111111111"),
        (Chain::Bitcoin, "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"),
    ] {
        assert!(
            NewWallet {
                address: address.into(),
                chain,
                nickname: None
            }
            .validated()
            .is_ok()
        );
        assert!(
            NewWallet {
                address: "invalid".into(),
                chain,
                nickname: None
            }
            .validated()
            .is_err()
        );
    }
    assert!(
        NewWallet {
            address: "0x52908400098527886E0F7030069857D2E4169Ee7".into(),
            chain: Chain::Ethereum,
            nickname: None
        }
        .validated()
        .is_err()
    );
}

#[test]
fn nickname_matches_javascript_trim_and_utf16_limits() {
    assert_eq!(
        NicknameUpdate::validated("\u{feff} Savings \u{00a0}")
            .unwrap()
            .nickname,
        "Savings"
    );
    assert!(NicknameUpdate::validated("   ").is_err());
    assert!(NicknameUpdate::validated(&"a".repeat(64)).is_ok());
    assert!(NicknameUpdate::validated(&"a".repeat(65)).is_err());
    assert!(NicknameUpdate::validated(&"🪙".repeat(32)).is_ok());
    assert!(NicknameUpdate::validated(&"🪙".repeat(33)).is_err());
}
