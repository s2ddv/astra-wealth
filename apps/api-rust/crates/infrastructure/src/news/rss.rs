use super::sources::RssSource;
use domain::news::*;
pub struct RssProvider {
    client: reqwest::Client,
    source: RssSource,
}
impl RssProvider {
    pub fn new(client: reqwest::Client, source: RssSource) -> Self {
        Self { client, source }
    }
}
pub fn plain_summary(value: &str) -> String {
    let clean = ammonia::Builder::new()
        .tags(std::collections::HashSet::new())
        .clean(value)
        .to_string();
    let decoded = html_escape::decode_html_entities(&clean);
    decoded
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(240)
        .collect()
}
pub fn parse(bytes: &[u8], source: RssSource) -> Result<Vec<NewsArticle>, NewsError> {
    let feed = feed_rs::parser::parse(bytes).map_err(|_| NewsError::Unavailable)?;
    Ok(feed
        .entries
        .into_iter()
        .filter_map(|entry| {
            let url = entry
                .links
                .iter()
                .find(|l| l.rel.as_deref().is_none_or(|rel| rel == "alternate"))
                .and_then(|l| normalized_url(&l.href))?;
            let title = plain_summary(&entry.title?.content);
            if title.is_empty() {
                return None;
            }
            let published = entry.published.or(entry.updated)?;
            let image = entry
                .media
                .iter()
                .flat_map(|m| m.content.iter())
                .filter(|m| {
                    m.content_type
                        .as_ref()
                        .is_none_or(|t| t.to_string().starts_with("image/"))
                })
                .find_map(|m| m.url.as_ref().and_then(|url| normalized_url(url.as_str())))
                .or_else(|| {
                    entry
                        .media
                        .iter()
                        .flat_map(|m| m.thumbnails.iter())
                        .find_map(|t| normalized_url(t.image.uri.as_str()))
                })
                .or_else(|| {
                    entry
                        .links
                        .iter()
                        .find(|l| {
                            l.rel.as_deref() == Some("enclosure")
                                && l.media_type
                                    .as_ref()
                                    .is_some_and(|t| t.starts_with("image/"))
                        })
                        .and_then(|l| normalized_url(&l.href))
                });
            Some(NewsArticle {
                id: url.clone(),
                title,
                summary: entry
                    .summary
                    .map(|s| plain_summary(&s.content))
                    .filter(|s| !s.is_empty()),
                url,
                image_url: image,
                source_name: source.name.into(),
                published_at: published.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                category: source.category,
                language: source.language,
                related_symbols: vec![],
            })
        })
        .take(100)
        .collect())
}
impl NewsProvider for RssProvider {
    fn fetch(&self, _query: NewsFilter) -> NewsFuture<'_, Vec<NewsArticle>> {
        Box::pin(async move {
            let mut response = self
                .client
                .get(self.source.url)
                .send()
                .await
                .map_err(|_| NewsError::Unavailable)?
                .error_for_status()
                .map_err(|_| NewsError::Unavailable)?;
            let mut body = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|_| NewsError::Unavailable)? {
                if body.len() + chunk.len() > 4 * 1024 * 1024 {
                    return Err(NewsError::Unavailable);
                }
                body.extend_from_slice(&chunk);
            }
            parse(&body, self.source)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_fixture_sanitizes_and_extracts_images() {
        let items = parse(
            include_bytes!("../../tests/fixtures/news.xml"),
            super::super::sources::SOURCES[0],
        )
        .unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "Bitcoin & mercados");
        assert_eq!(items[0].summary.as_deref(), Some("Olá mundo & economia."));
        assert_eq!(
            items[0].image_url.as_deref(),
            Some("https://example.com/image.jpg")
        );
        assert_eq!(
            items[1].image_url.as_deref(),
            Some("https://example.com/two.png")
        );
        assert_eq!(items[0].published_at, "2026-09-28T12:00:00Z");
        assert!(items[0].related_symbols.is_empty());
        assert!(parse(b"broken", super::super::sources::SOURCES[0]).is_err());
    }
    #[test]
    fn summaries_are_unicode_bounded() {
        assert_eq!(plain_summary(&"é".repeat(300)).chars().count(), 240);
    }
}
