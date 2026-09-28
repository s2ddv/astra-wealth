use super::{cache::RedisNewsCache, rss::plain_summary};
use domain::news::*;
use serde::Deserialize;
use std::sync::Arc;
pub struct NewsDataProvider {
    client: reqwest::Client,
    cache: Arc<RedisNewsCache>,
    key: String,
    endpoint: &'static str,
}
impl NewsDataProvider {
    pub fn new(
        client: reqwest::Client,
        cache: Arc<RedisNewsCache>,
        key: String,
        endpoint: &'static str,
    ) -> Self {
        Self {
            client,
            cache,
            key,
            endpoint,
        }
    }
}
#[derive(Deserialize)]
struct Payload {
    status: String,
    results: Vec<Article>,
}
#[derive(Deserialize)]
struct Article {
    title: Option<String>,
    description: Option<String>,
    link: Option<String>,
    image_url: Option<String>,
    source_name: Option<String>,
    source_id: Option<String>,
    #[serde(rename = "pubDate")]
    published: Option<String>,
    language: Option<String>,
}
fn parse(payload: Payload, category: NewsCategory) -> Result<Vec<NewsArticle>, NewsError> {
    if payload.status != "success" {
        return Err(NewsError::Unavailable);
    }
    Ok(payload
        .results
        .into_iter()
        .filter_map(|a| {
            let url = normalized_url(&a.link?)?;
            let language = match a.language?.as_str() {
                "portuguese" => NewsLanguage::Pt,
                "english" => NewsLanguage::En,
                _ => return None,
            };
            let date = chrono::NaiveDateTime::parse_from_str(&a.published?, "%Y-%m-%d %H:%M:%S")
                .ok()?
                .and_utc();
            let title = plain_summary(&a.title?);
            if title.is_empty() {
                return None;
            }
            Some(NewsArticle {
                id: url.clone(),
                title,
                summary: a.description.map(|s| plain_summary(&s)),
                url,
                image_url: a.image_url.and_then(|s| normalized_url(&s)),
                source_name: a.source_name.or(a.source_id)?,
                published_at: date.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                category,
                language,
                related_symbols: vec![],
            })
        })
        .collect())
}
impl NewsProvider for NewsDataProvider {
    fn fetch(&self, _query: NewsFilter) -> NewsFuture<'_, Vec<NewsArticle>> {
        Box::pin(async move {
            let key = format!("news:v1:newsdata:{}", self.endpoint);
            if let Some(items) = self.cache.read(&key).await? {
                return Ok(items);
            }
            if !self.cache.reserve_credit(self.endpoint).await? {
                return self.cache.read(&key).await?.ok_or(NewsError::Unavailable);
            }
            // Never propagate reqwest errors: their Display may include the secret URL.
            let mut url = url::Url::parse(&format!("https://newsdata.io/api/1/{}", self.endpoint))
                .map_err(|_| NewsError::Unavailable)?;
            url.query_pairs_mut().extend_pairs([
                ("apikey", self.key.as_str()),
                ("language", "pt,en"),
                ("size", "10"),
            ]);
            let payload: Payload = self
                .client
                .get(url)
                .send()
                .await
                .map_err(|_| NewsError::Unavailable)?
                .error_for_status()
                .map_err(|_| NewsError::Unavailable)?
                .json()
                .await
                .map_err(|_| NewsError::Unavailable)?;
            let articles = parse(
                payload,
                if self.endpoint == "crypto" {
                    NewsCategory::Crypto
                } else {
                    NewsCategory::Macro
                },
            )?;
            self.cache.write(&key, &articles, 1200).await?;
            Ok(articles)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maps_newsdata_without_full_text_or_unsupported_languages() {
        let payload = serde_json::from_str(r#"{"status":"success","results":[{"title":"Economia","description":"<b>Resumo</b>","link":"https://example.com/a","image_url":"javascript:bad","source_name":"Fonte","pubDate":"2026-09-28 12:00:00","language":"portuguese","content":"never expose full text"},{"title":"Other","link":"https://example.com/b","pubDate":"2026-09-28 12:00:00","language":"spanish"}]}"#).unwrap();
        let articles = parse(payload, NewsCategory::Macro).unwrap();
        assert_eq!(articles.len(), 1);
        assert_eq!(articles[0].summary.as_deref(), Some("Resumo"));
        assert!(articles[0].image_url.is_none());
        assert_eq!(articles[0].language, NewsLanguage::Pt);
    }
}
