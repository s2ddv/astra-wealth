pub mod cache;
pub mod newsdata;
pub mod rss;
pub mod sources;
use domain::news::NewsProvider;
use std::{sync::Arc, time::Duration};

pub fn news_service(
    pool: deadpool_redis::Pool,
    key: Option<String>,
) -> anyhow::Result<application::news::NewsService> {
    let client = reqwest::Client::builder()
        .retry(reqwest::retry::never())
        .timeout(Duration::from_secs(5))
        .user_agent("ZoraWealth-News/1.0")
        .build()?;
    let cache = Arc::new(cache::RedisNewsCache::new(pool));
    let mut providers: Vec<Arc<dyn NewsProvider>> = sources::SOURCES
        .iter()
        .map(|source| {
            Arc::new(rss::RssProvider::new(client.clone(), *source)) as Arc<dyn NewsProvider>
        })
        .collect();
    if let Some(key) = key.filter(|key| !key.trim().is_empty()) {
        // One reserved credit must correspond to exactly one HTTP attempt.
        let paid_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .retry(reqwest::retry::never())
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("ZoraWealth-News/1.0")
            .build()?;
        for endpoint in ["crypto", "market"] {
            providers.push(Arc::new(newsdata::NewsDataProvider::new(
                paid_client.clone(),
                cache.clone(),
                key.clone(),
                endpoint,
            )));
        }
    } else {
        tracing::warn!("NEWSDATA_API_KEY absent; news uses RSS only");
    }
    Ok(application::news::NewsService::new(providers, cache))
}
