//! Public crypto market boundary; every upstream operation stays in application.
use application::market::CryptoMarketService;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use domain::{
    asset_icon::{IconError, IconResponse},
    market::{MarketCoin, MarketQuery},
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MarketCoinDto {
    pub id: String,
    pub symbol: String,
    pub name: String,
    pub image: Option<String>,
    pub current_price: Option<f64>,
    pub price_change_percentage_24h: Option<f64>,
    pub market_cap: Option<f64>,
    pub volume_24h: Option<f64>,
    pub rank: Option<u32>,
    pub updated_at: Option<String>,
}
impl From<(MarketCoin, IconResponse)> for MarketCoinDto {
    fn from((coin, icon): (MarketCoin, IconResponse)) -> Self {
        Self {
            id: coin.id,
            symbol: coin.symbol,
            name: coin.name,
            image: icon.url.map(|url| url.as_str().into()),
            current_price: coin.price_usd,
            price_change_percentage_24h: coin.change_24h,
            market_cap: coin.market_cap,
            volume_24h: coin.volume_24h,
            rank: coin.rank,
            updated_at: coin.updated_at,
        }
    }
}
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MarketPageDto {
    pub data: Vec<MarketCoinDto>,
    pub page: u32,
    pub per_page: u32,
    pub has_more: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PageQuery {
    page: Option<u32>,
    per_page: Option<u32>,
    ids: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpotQuery {
    limit: Option<u32>,
}

pub struct MarketError(IconError);
impl From<IconError> for MarketError {
    fn from(error: IconError) -> Self {
        Self(error)
    }
}
impl IntoResponse for MarketError {
    fn into_response(self) -> Response {
        let (status, message) = match self.0 {
            IconError::InvalidReference => (StatusCode::BAD_REQUEST, "Invalid market query"),
            IconError::NotFound => (StatusCode::NOT_FOUND, "Asset not found"),
            IconError::RateLimited => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Market rate limit reached; retry later",
            ),
            IconError::Unavailable => (StatusCode::BAD_GATEWAY, "Market data unavailable"),
        };
        let mut response = (status, Json(serde_json::json!({"error":message}))).into_response();
        if status == StatusCode::SERVICE_UNAVAILABLE {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, "60".parse().unwrap());
        }
        response
    }
}
async fn markets(
    State(service): State<CryptoMarketService>,
    Query(query): Query<PageQuery>,
) -> Result<Json<MarketPageDto>, MarketError> {
    let query = MarketQuery {
        page: query.page.unwrap_or(1),
        per_page: query.per_page.unwrap_or(50),
        ids: query
            .ids
            .map(|ids| ids.split(',').map(str::to_owned).collect())
            .unwrap_or_default(),
    };
    let page = query.page;
    let per_page = query.per_page;
    let data: Vec<_> = service
        .market_page(query)
        .await?
        .into_iter()
        .map(MarketCoinDto::from)
        .collect();
    let has_more = data.len() == per_page as usize;
    Ok(Json(MarketPageDto {
        data,
        page,
        per_page,
        has_more,
    }))
}
async fn spot(
    State(service): State<CryptoMarketService>,
    Query(query): Query<SpotQuery>,
) -> Result<Json<Vec<MarketCoinDto>>, MarketError> {
    let data = service
        .market_page(MarketQuery {
            per_page: query.limit.unwrap_or(10),
            ..MarketQuery::default()
        })
        .await?;
    Ok(Json(data.into_iter().map(MarketCoinDto::from).collect()))
}
async fn trending(
    State(service): State<CryptoMarketService>,
) -> Result<Json<Vec<MarketCoinDto>>, MarketError> {
    Ok(Json(
        service
            .trending()
            .await?
            .into_iter()
            .map(MarketCoinDto::from)
            .collect(),
    ))
}
async fn coin(
    State(service): State<CryptoMarketService>,
    Path(id): Path<String>,
) -> Result<Json<MarketCoinDto>, MarketError> {
    Ok(Json(service.coin(&id).await?.into()))
}
pub fn router(service: CryptoMarketService) -> Router {
    Router::new()
        .route("/v1/market/coins", get(markets))
        .route("/v1/market/coins/{id}", get(coin))
        .route("/v1/market/spot", get(spot))
        .route("/v1/market/trending", get(trending))
        .with_state(service)
}
