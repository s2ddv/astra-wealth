# RSS source verification — 2026-09-28

Only feed-supplied titles, short descriptions, timestamps, image references and
original links are used. Article pages are never fetched. Source names remain
visible. The configuration is `crates/infrastructure/src/news/sources.rs`.

## Enabled

- **Crypto Briefing (crypto, EN)**: <https://cryptobriefing.com/feed/>.
  Discovered in the official [RSS directory](https://cryptobriefing.com/feeds/)
  and its HTML alternate link. Live XML validation: RSS, 30 items.
  Reviewed the linked [disclaimer](https://cryptobriefing.com/disclaimer/);
  no prohibition of headline/description feed aggregation found there.
- **CriptoFácil (crypto, PT)**: <https://www.criptofacil.com/feed/>.
  Discovered through the official homepage's `rel=alternate` RSS link.
  Live XML validation: RSS, 15 items. Reviewed linked
  [privacy policy](https://www.criptofacil.com/politica-de-privacidade/)
  and [editorial policy](https://www.criptofacil.com/politica-editorial/);
  no prohibition of this feed usage found in those policies.
- **Agência Brasil / Economia (macro, PT)**:
  <https://agenciabrasil.ebc.com.br/rss/economia/feed.xml>.
  Linked from the official [feed directory](https://agenciabrasil.ebc.com.br/feed/).
  Live XML validation: RSS, 10 items. Its [about page](https://agenciabrasil.ebc.com.br/sobre)
  permits journalistic reproduction with source attribution.
- **Federal Reserve / Monetary Policy (macro, EN)**:
  <https://www.federalreserve.gov/feeds/press_monetary.xml>.
  Linked from the official [RSS directory](https://www.federalreserve.gov/feeds/feeds.htm),
  which describes aggregators displaying links, headlines and summaries.
  Live XML validation: RSS, 15 items. This is an institutional primary source,
  not a general financial newspaper; the interface identifies it accordingly.

The checks above record public evidence, not a negotiated content license.
Review publisher policy changes before expanding usage beyond RSS excerpts.

## Excluded

- **Cointelegraph**: [terms](https://cointelegraph.com/terms-and-privacy)
  prohibit unauthorized redistribution and independent content pipelines.
- **Decrypt**: [terms](https://decrypt.co/terms-of-service) cover RSS and restrict
  automated collection and commercial use without authorization.
- **CoinDesk**: [RSS announcement](https://www.coindesk.com/coindesk-news/2021/09/17/coindesk-rss)
  exists, but current [terms](https://www.coindesk.com/terms) restrict redistribution.
  The older 2021 media kit's free headline-link offering was not treated as a
  current license overriding these terms.
- **Bitcoin Magazine**: homepage advertises <https://bitcoinmagazine.com/feed>
  (validated RSS, 10 items). Excluded because [terms](https://bitcoinmagazine.com/terms-of-use)
  require consent for redistribution; also links to [BTC terms](https://www.b.tc/terms).
- **Livecoins**: official homepage advertises RSS, but [terms](https://livecoins.com.br/termos-de-uso/)
  explicitly restrict republication, caching and commercial use without permission.
- **Portal do Bitcoin / UOL**: no RSS alternate link found in the current official
  homepage; no validated public feed selected. UOL's [copyright notice](https://noticias.uol.com.br/regras/aviso-de-direitos-autorais/)
  also restricts commercial reproduction/caching.
- **InfoMoney**: official homepage advertises <https://www.infomoney.com.br/feed/>
  (validated RSS, 10 items), but this is a general feed and reuse terms could not
  be established in this check. Chose the explicitly attributed Agência Brasil
  economy feed instead. This does not assert InfoMoney prohibits all RSS usage.

NewsData uses its licensed API separately; these exclusions concern direct RSS
configuration, not source metadata returned by NewsData.
