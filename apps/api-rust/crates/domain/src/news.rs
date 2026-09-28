use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use garde::Validate;
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin};

pub type NewsFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, NewsError>> + Send + 'a>>;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NewsCategory {
    Crypto,
    Macro,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NewsLanguage {
    Pt,
    En,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsArticle {
    pub id: String,
    pub title: String,
    pub summary: Option<String>,
    pub url: String,
    pub image_url: Option<String>,
    pub source_name: String,
    /// UTC RFC3339, normalized at the provider boundary.
    pub published_at: String,
    pub category: NewsCategory,
    pub language: NewsLanguage,
    pub related_symbols: Vec<String>,
}
#[derive(Debug, thiserror::Error)]
pub enum NewsError {
    #[error("Invalid news query")]
    InvalidQuery,
    #[error("News temporarily unavailable")]
    Unavailable,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewsFilter {
    pub category: Option<NewsCategory>,
    pub language: Option<NewsLanguage>,
}
impl NewsFilter {
    pub fn key(&self) -> String {
        format!(
            "{}:{}",
            match self.category {
                None => "all",
                Some(NewsCategory::Crypto) => "crypto",
                Some(NewsCategory::Macro) => "macro",
            },
            match self.language {
                None => "all",
                Some(NewsLanguage::Pt) => "pt",
                Some(NewsLanguage::En) => "en",
            }
        )
    }
    pub fn matches(&self, article: &NewsArticle) -> bool {
        self.category.is_none_or(|v| v == article.category)
            && self.language.is_none_or(|v| v == article.language)
    }
}
#[derive(Debug, Deserialize, Validate)]
#[serde(default, deny_unknown_fields)]
pub struct NewsQuery {
    #[garde(pattern("^(all|crypto|macro)$"))]
    pub category: String,
    #[garde(pattern("^(all|pt|en)$"))]
    pub lang: String,
    #[garde(range(min = 1, max = 50))]
    pub limit: usize,
    #[garde(length(max = 128))]
    pub cursor: Option<String>,
}
impl Default for NewsQuery {
    fn default() -> Self {
        Self {
            category: "all".into(),
            lang: "all".into(),
            limit: 20,
            cursor: None,
        }
    }
}
impl NewsQuery {
    pub fn parse(&self) -> Result<(NewsFilter, usize), NewsError> {
        self.validate().map_err(|_| NewsError::InvalidQuery)?;
        let filter = NewsFilter {
            category: match self.category.as_str() {
                "crypto" => Some(NewsCategory::Crypto),
                "macro" => Some(NewsCategory::Macro),
                _ => None,
            },
            language: match self.lang.as_str() {
                "pt" => Some(NewsLanguage::Pt),
                "en" => Some(NewsLanguage::En),
                _ => None,
            },
        };
        let offset = match &self.cursor {
            None => 0,
            Some(cursor) => {
                let decoded = URL_SAFE_NO_PAD
                    .decode(cursor)
                    .map_err(|_| NewsError::InvalidQuery)?;
                let decoded = String::from_utf8(decoded).map_err(|_| NewsError::InvalidQuery)?;
                decoded
                    .strip_prefix(&format!("v1:{}:", filter.key()))
                    .ok_or(NewsError::InvalidQuery)?
                    .parse::<usize>()
                    .map_err(|_| NewsError::InvalidQuery)?
            }
        };
        if offset > 200 {
            return Err(NewsError::InvalidQuery);
        }
        Ok((filter, offset))
    }
}
pub fn cursor(filter: &NewsFilter, offset: usize) -> String {
    URL_SAFE_NO_PAD.encode(format!("v1:{}:{offset}", filter.key()))
}
pub trait NewsProvider: Send + Sync {
    fn fetch(&self, query: NewsFilter) -> NewsFuture<'_, Vec<NewsArticle>>;
}
pub trait NewsCache: Send + Sync {
    fn get<'a>(&'a self, key: &'a str, stale: bool) -> NewsFuture<'a, Option<Vec<NewsArticle>>>;
    fn put<'a>(&'a self, key: &'a str, articles: &'a [NewsArticle]) -> NewsFuture<'a, ()>;
}
pub fn normalized_url(raw: &str) -> Option<String> {
    let mut url = url::Url::parse(raw).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    url.set_fragment(None);
    let mut pairs: Vec<_> = url
        .query_pairs()
        .filter(|(k, _)| !k.to_ascii_lowercase().starts_with("utm_"))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    pairs.sort();
    url.set_query(None);
    if !pairs.is_empty() {
        url.query_pairs_mut().extend_pairs(pairs);
    }
    Some(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_limits_filters_and_bound_cursor() {
        assert!(NewsQuery::default().parse().is_ok());
        for limit in [0, 51, usize::MAX] {
            assert!(
                NewsQuery {
                    limit,
                    ..Default::default()
                }
                .parse()
                .is_err()
            );
        }
        assert!(
            NewsQuery {
                category: "stocks".into(),
                ..Default::default()
            }
            .parse()
            .is_err()
        );
        assert!(
            NewsQuery {
                lang: "es".into(),
                ..Default::default()
            }
            .parse()
            .is_err()
        );
        for raw in ["invalid".into(), cursor(&NewsFilter::default(), 201)] {
            assert!(
                NewsQuery {
                    cursor: Some(raw),
                    ..Default::default()
                }
                .parse()
                .is_err()
            );
        }
        let next = cursor(&NewsFilter::default(), 20);
        assert_eq!(
            NewsQuery {
                cursor: Some(next.clone()),
                ..Default::default()
            }
            .parse()
            .unwrap()
            .1,
            20
        );
        assert!(
            NewsQuery {
                cursor: Some(next),
                category: "crypto".into(),
                ..Default::default()
            }
            .parse()
            .is_err()
        );
    }
    #[test]
    fn canonicalizes_url_without_tracking_or_fragment() {
        assert_eq!(
            normalized_url("https://EXAMPLE.com/a?z=2&utm_source=x&a=1#fragment").unwrap(),
            "https://example.com/a?a=1&z=2"
        );
        assert!(normalized_url("javascript:alert(1)").is_none());
    }
}
