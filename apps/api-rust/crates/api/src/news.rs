use crate::auth::{AuthState, AuthenticatedUser};
use application::news::NewsService;
use axum::{
    Extension, Json, Router,
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use domain::news::{NewsArticle, NewsCategory, NewsError, NewsLanguage, NewsQuery};
use serde::Serialize;
use ts_rs::TS;

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NewsArticleDto {
    pub id: String,
    pub title: String,
    pub summary: Option<String>,
    pub url: String,
    pub image_url: Option<String>,
    pub source_name: String,
    pub published_at: String,
    #[ts(type = "'crypto' | 'macro'")]
    pub category: &'static str,
    #[ts(type = "'pt' | 'en'")]
    pub language: &'static str,
    pub related_symbols: Vec<String>,
}
impl From<NewsArticle> for NewsArticleDto {
    fn from(a: NewsArticle) -> Self {
        Self {
            id: a.id,
            title: a.title,
            summary: a.summary,
            url: a.url,
            image_url: a.image_url,
            source_name: a.source_name,
            published_at: a.published_at,
            category: match a.category {
                NewsCategory::Crypto => "crypto",
                NewsCategory::Macro => "macro",
            },
            language: match a.language {
                NewsLanguage::Pt => "pt",
                NewsLanguage::En => "en",
            },
            related_symbols: a.related_symbols,
        }
    }
}
#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NewsPageDto {
    pub data: Vec<NewsArticleDto>,
    pub next_cursor: Option<String>,
    pub stale: bool,
}
pub struct NewsRejection(NewsError);
impl From<NewsError> for NewsRejection {
    fn from(error: NewsError) -> Self {
        Self(error)
    }
}
impl IntoResponse for NewsRejection {
    fn into_response(self) -> Response {
        (
            match self.0 {
                NewsError::InvalidQuery => StatusCode::BAD_REQUEST,
                NewsError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            },
            Json(serde_json::json!({ "error": self.0.to_string() })),
        )
            .into_response()
    }
}
async fn news(
    Extension(service): Extension<NewsService>,
    _user: AuthenticatedUser,
    Query(query): Query<NewsQuery>,
) -> Result<Json<NewsPageDto>, NewsRejection> {
    let page = service.page(query).await?;
    Ok(Json(NewsPageDto {
        data: page.data.into_iter().map(Into::into).collect(),
        next_cursor: page.next_cursor,
        stale: page.stale,
    }))
}
pub fn router(auth: AuthState, news_service: NewsService) -> Router {
    Router::new()
        .route("/v1/news", get(news))
        .layer(Extension(news_service))
        .with_state(auth)
}
