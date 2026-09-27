//! Export-only contract. No icon or stock route is registered in this stage.
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct IconResponse {
    // Approved HTTPS URL or downloaded PNG data URL; null means frontend placeholder.
    pub url: Option<String>,
    pub symbol: String,
}
impl From<domain::asset_icon::IconResponse> for IconResponse {
    fn from(icon: domain::asset_icon::IconResponse) -> Self {
        Self {
            url: icon.url.map(|url| url.as_str().to_owned()),
            symbol: icon.symbol,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn placeholder_contract_serializes_as_null_and_symbol() {
        let dto = IconResponse::from(domain::asset_icon::IconResponse {
            url: None,
            symbol: "BTC".into(),
        });
        assert_eq!(
            serde_json::to_value(dto).unwrap(),
            serde_json::json!({"url": null, "symbol": "BTC"})
        );
    }
}
