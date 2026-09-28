//! Official RSS discovery and selection evidence: docs/news-sources.md.
use domain::news::{NewsCategory, NewsLanguage};
#[derive(Clone, Copy)]
pub struct RssSource {
    pub name: &'static str,
    pub url: &'static str,
    pub category: NewsCategory,
    pub language: NewsLanguage,
}
pub const SOURCES: &[RssSource] = &[
    RssSource {
        name: "Crypto Briefing",
        url: "https://cryptobriefing.com/feed/",
        category: NewsCategory::Crypto,
        language: NewsLanguage::En,
    },
    RssSource {
        name: "CriptoFácil",
        url: "https://www.criptofacil.com/feed/",
        category: NewsCategory::Crypto,
        language: NewsLanguage::Pt,
    },
    RssSource {
        name: "Agência Brasil",
        url: "https://agenciabrasil.ebc.com.br/rss/economia/feed.xml",
        category: NewsCategory::Macro,
        language: NewsLanguage::Pt,
    },
    RssSource {
        name: "Federal Reserve",
        url: "https://www.federalreserve.gov/feeds/press_monetary.xml",
        category: NewsCategory::Macro,
        language: NewsLanguage::En,
    },
];
