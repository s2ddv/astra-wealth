use domain::news::*;
use futures_util::future::join_all;
use std::{collections::HashSet, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct NewsService {
    providers: Vec<Arc<dyn NewsProvider>>,
    cache: Arc<dyn NewsCache>,
    // Shared by all handler clones. A second cache check after acquiring the gate
    // coalesces concurrent misses, including requests for different filters.
    flight: Arc<Mutex<()>>,
}
pub struct NewsPage {
    pub data: Vec<NewsArticle>,
    pub next_cursor: Option<String>,
    pub stale: bool,
}
impl NewsService {
    pub fn new(providers: Vec<Arc<dyn NewsProvider>>, cache: Arc<dyn NewsCache>) -> Self {
        Self {
            providers,
            cache,
            flight: Arc::new(Mutex::new(())),
        }
    }
    pub async fn page(&self, query: NewsQuery) -> Result<NewsPage, NewsError> {
        let (filter, offset) = query.parse()?;
        let (articles, stale) = self.articles(&filter).await?;
        let end = (offset + query.limit).min(articles.len());
        Ok(NewsPage {
            data: articles
                .iter()
                .skip(offset)
                .take(query.limit)
                .cloned()
                .collect(),
            next_cursor: (end < articles.len()).then(|| cursor(&filter, end)),
            stale,
        })
    }
    async fn articles(&self, filter: &NewsFilter) -> Result<(Vec<NewsArticle>, bool), NewsError> {
        let key = filter.key();
        if let Ok(Some(items)) = self.cache.get(&key, false).await {
            return Ok((items, false));
        }
        let _guard = self.flight.lock().await;
        if let Ok(Some(items)) = self.cache.get(&key, false).await {
            return Ok((items, false));
        }
        let results = join_all(self.providers.iter().map(|p| p.fetch(filter.clone()))).await;
        let mut articles = Vec::new();
        let mut success = false;
        for result in results {
            match result {
                Ok(items) => {
                    success = true;
                    articles.extend(items.into_iter().filter(|a| filter.matches(a)));
                }
                Err(_) => tracing::warn!("News provider unavailable"),
            }
        }
        if !success {
            return self
                .cache
                .get(&key, true)
                .await?
                .map(|items| (items, true))
                .ok_or(NewsError::Unavailable);
        }
        let articles = merge(articles);
        if self.cache.put(&key, &articles).await.is_err() {
            tracing::warn!("News cache write unavailable");
        }
        Ok((articles, false))
    }
}
pub fn merge(mut articles: Vec<NewsArticle>) -> Vec<NewsArticle> {
    articles.sort_by(|a, b| {
        b.published_at
            .cmp(&a.published_at)
            .then_with(|| a.id.cmp(&b.id))
    });
    let mut urls = HashSet::new();
    let mut titles = HashSet::new();
    articles.retain_mut(|a| {
        let Some(url) = normalized_url(&a.url) else {
            return false;
        };
        let title: String = a
            .title
            .chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect();
        if title.is_empty() || urls.contains(&url) || titles.contains(&title) {
            return false;
        }
        urls.insert(url.clone());
        titles.insert(title);
        a.url = url;
        true
    });
    articles.truncate(200);
    articles
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Mutex as StdMutex,
        atomic::{AtomicUsize, Ordering},
    };
    #[derive(Default)]
    struct Cache {
        fresh: StdMutex<Option<Vec<NewsArticle>>>,
        stale: StdMutex<Option<Vec<NewsArticle>>>,
    }
    impl NewsCache for Cache {
        fn get<'a>(&'a self, _: &'a str, stale: bool) -> NewsFuture<'a, Option<Vec<NewsArticle>>> {
            Box::pin(async move {
                Ok(if stale { &self.stale } else { &self.fresh }
                    .lock()
                    .unwrap()
                    .clone())
            })
        }
        fn put<'a>(&'a self, _: &'a str, items: &'a [NewsArticle]) -> NewsFuture<'a, ()> {
            Box::pin(async move {
                *self.fresh.lock().unwrap() = Some(items.to_vec());
                Ok(())
            })
        }
    }
    struct Provider {
        calls: Arc<AtomicUsize>,
        fail: bool,
    }
    impl NewsProvider for Provider {
        fn fetch(&self, _: NewsFilter) -> NewsFuture<'_, Vec<NewsArticle>> {
            Box::pin(async move {
                self.calls.fetch_add(1, Ordering::SeqCst);
                if self.fail {
                    Err(NewsError::Unavailable)
                } else {
                    Ok(vec![
                        article("one", "First", "2026-09-28T12:00:00Z"),
                        article("two", "Second", "2026-09-27T12:00:00Z"),
                    ])
                }
            })
        }
    }
    fn article(url: &str, title: &str, date: &str) -> NewsArticle {
        NewsArticle {
            id: url.into(),
            title: title.into(),
            summary: None,
            url: format!("https://example.com/{url}"),
            image_url: None,
            source_name: "Test".into(),
            published_at: date.into(),
            category: NewsCategory::Crypto,
            language: NewsLanguage::En,
            related_symbols: vec![],
        }
    }
    #[test]
    fn deduplicates_by_url_or_title_and_sorts() {
        let items = merge(vec![
            article("one", "Old", "2026-09-27T12:00:00Z"),
            article("one?utm_source=a#b", "New", "2026-09-28T12:00:00Z"),
            article("other", " NEW! ", "2026-09-28T11:00:00Z"),
            article("two", "Unique", "2026-09-26T12:00:00Z"),
        ]);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "New");
        assert_eq!(items[0].url, "https://example.com/one");
    }
    #[tokio::test]
    async fn partial_failure_cache_hit_pagination_and_single_flight() {
        let calls = Arc::new(AtomicUsize::new(0));
        let service = NewsService::new(
            vec![
                Arc::new(Provider {
                    calls: calls.clone(),
                    fail: true,
                }),
                Arc::new(Provider {
                    calls: calls.clone(),
                    fail: false,
                }),
            ],
            Arc::new(Cache::default()),
        );
        let (a, b) = tokio::join!(
            service.page(NewsQuery {
                limit: 1,
                ..Default::default()
            }),
            service.page(NewsQuery::default())
        );
        let a = a.unwrap();
        assert_eq!(a.data.len(), 1);
        assert_eq!(b.unwrap().data.len(), 2);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        let page = service
            .page(NewsQuery {
                cursor: a.next_cursor,
                limit: 1,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(page.data[0].title, "Second");
        assert!(page.next_cursor.is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
    #[tokio::test]
    async fn all_failed_uses_stale_or_returns_error() {
        let cache = Arc::new(Cache::default());
        let service = NewsService::new(
            vec![Arc::new(Provider {
                calls: Arc::new(AtomicUsize::new(0)),
                fail: true,
            })],
            cache.clone(),
        );
        assert!(service.page(NewsQuery::default()).await.is_err());
        *cache.stale.lock().unwrap() = Some(vec![article("old", "Cached", "2026-09-28T12:00:00Z")]);
        let page = service.page(NewsQuery::default()).await.unwrap();
        assert!(page.stale);
        assert_eq!(page.data[0].title, "Cached");
    }
}
