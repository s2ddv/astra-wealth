use application::public_wallet::{NativeBalanceProvider, display_wei};
use domain::asset_icon::{IconError, IconFuture};
use reqwest::Client;
use serde_json::{Value, json};

pub enum ProviderKind {
    Alchemy,
    Moralis,
}
/// Keys and URLs deliberately have no Debug implementation.
pub struct PublicBalanceProvider {
    client: Client,
    base: String,
    key: Option<String>,
    kind: ProviderKind,
}
impl PublicBalanceProvider {
    pub fn new(client: Client, kind: ProviderKind, key: Option<String>) -> Self {
        let base = match kind {
            ProviderKind::Alchemy => "https://eth-mainnet.g.alchemy.com/v2",
            ProviderKind::Moralis => "https://deep-index.moralis.io/api/v2.2",
        };
        Self {
            client,
            base: base.into(),
            key,
            kind,
        }
    }
}
impl NativeBalanceProvider for PublicBalanceProvider {
    fn balance<'a>(&'a self, address: &'a str) -> IconFuture<'a, String> {
        Box::pin(async move {
            let key = self
                .key
                .as_deref()
                .filter(|key| !key.trim().is_empty())
                .ok_or(IconError::Unavailable)?;
            let request = match self.kind {
                ProviderKind::Alchemy => self.client.post(format!("{}/{key}", self.base)).json(&json!({
                    "jsonrpc":"2.0", "id":1, "method":"eth_getBalance", "params":[address,"latest"]
                })),
                ProviderKind::Moralis => self.client.get(format!("{}/{address}/balance?chain=eth", self.base)).header("X-API-Key", key),
            };
            let mut response = request.send().await.map_err(|_| IconError::Unavailable)?;
            if !response.status().is_success() {
                return Err(IconError::Unavailable);
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|_| IconError::Unavailable)? {
                if bytes.len() + chunk.len() > 65536 {
                    return Err(IconError::Unavailable);
                }
                bytes.extend_from_slice(&chunk);
            }
            let value: Value =
                serde_json::from_slice(&bytes).map_err(|_| IconError::Unavailable)?;
            let (field, radix) = match self.kind {
                ProviderKind::Alchemy => ("result", 16),
                ProviderKind::Moralis => ("balance", 10),
            };
            display_wei(value[field].as_str().ok_or(IconError::Unavailable)?, radix)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Json, Router,
        extract::{Path, Query},
        http::HeaderMap,
        routing::{get, post},
    };
    use std::collections::HashMap;
    #[tokio::test]
    async fn provider_requests_and_units_match_legacy_contract() {
        async fn alchemy(Path(key): Path<String>, Json(body): Json<Value>) -> Json<Value> {
            assert_eq!(key, "synthetic");
            assert_eq!(body["method"], "eth_getBalance");
            assert_eq!(body["params"][1], "latest");
            Json(json!({"result":"0xde0b6b3a7640000"}))
        }
        async fn moralis(
            Path(address): Path<String>,
            Query(query): Query<HashMap<String, String>>,
            headers: HeaderMap,
        ) -> Json<Value> {
            assert_eq!(address, "0x0000000000000000000000000000000000000001");
            assert_eq!(query["chain"], "eth");
            assert_eq!(headers["x-api-key"], "synthetic");
            Json(json!({"balance":"1000000000000000000"}))
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let app = Router::new()
            .route("/{key}", post(alchemy))
            .route("/{address}/balance", get(moralis));
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        for kind in [ProviderKind::Alchemy, ProviderKind::Moralis] {
            let mut provider = PublicBalanceProvider::new(
                crate::asset_icon::asset_http_client().unwrap(),
                kind,
                Some("synthetic".into()),
            );
            provider.base = base.clone();
            assert_eq!(
                provider
                    .balance("0x0000000000000000000000000000000000000001")
                    .await
                    .unwrap(),
                "1.000000"
            );
        }
        task.abort();
    }
}
