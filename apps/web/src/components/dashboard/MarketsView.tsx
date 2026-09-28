"use client";

import { useEffect, useRef, useState } from "react";
import { useMarkets, useTrendingMarkets, useWatchlistMarkets, type MarketCoinDto } from "@/hooks/useMarkets";
import { changeToneClass, formatPercent } from "@/lib/format";
import { defaultWatchlist, mockMarketAssets } from "@/mocks/markets";
import { ASSET_CLASS_LABELS, formatMarketPrice, type AssetClass, type MarketInstrument } from "@/types/markets";
import { AssetBadge, MarketTable } from "./MarketTable";
import { MarketDetails } from "./MarketDetails";
import { WatchlistButton } from "./WatchlistButton";

const STORAGE_KEY = "zora.markets.watchlist.v1";
const FOCUS = "focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-primary";
const classes = Object.entries(ASSET_CLASS_LABELS) as [AssetClass, string][];
const normalize = (value: string) => value.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase().trim();
const demos = mockMarketAssets.filter((asset) => asset.assetClass !== "crypto");
const legacyIds: Record<string, string> = { "crypto:btc": "crypto:bitcoin", "crypto:eth": "crypto:ethereum", "crypto:sol": "crypto:solana", "crypto:link": "crypto:chainlink", "crypto:usdt": "crypto:tether" };
function instrument(coin: MarketCoinDto): MarketInstrument {
  return { id: `crypto:${coin.id}`, symbol: coin.symbol.toUpperCase(), name: coin.name, assetClass: "crypto", venue: "CoinGecko", currency: "USD", price: coin.currentPrice, change: coin.priceChangePercentage24h, image: coin.image, updatedAt: coin.updatedAt };
}

export function MarketsView() {
  const [watchlist, setWatchlist] = useState<string[]>(defaultWatchlist.map((id) => legacyIds[id] ?? id));
  const [ready, setReady] = useState(false);
  const [storageError, setStorageError] = useState(false);
  const [query, setQuery] = useState("");
  const [assetClass, setAssetClass] = useState<AssetClass | "all">("all");
  const [view, setView] = useState<"all" | "watchlist">("all");
  const [sort, setSort] = useState("default");
  const [page, setPage] = useState(1);
  const [message, setMessage] = useState("");
  const [selected, setSelected] = useState<string | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const market = useMarkets(page);
  const trending = useTrendingMarkets();
  const cryptoIds = watchlist.filter((id) => id.startsWith("crypto:")).map((id) => id.slice(7));
  const savedMarket = useWatchlistMarkets(cryptoIds, ready);

  useEffect(() => {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved !== null) {
        const ids: unknown = JSON.parse(saved);
        if (!Array.isArray(ids) || !ids.every((id) => typeof id === "string")) throw new Error("Invalid watchlist");
        setWatchlist([...new Set(ids.map((id) => legacyIds[id] ?? id).filter((id) => /^crypto:[a-z0-9][a-z0-9-]{0,99}$/.test(id) || demos.some((asset) => asset.id === id)))].slice(0, 100));
      }
    } catch { setStorageError(true); }
    setReady(true);
  }, []);

  const pageAssets = (market.data?.data ?? []).map(instrument);
  const assets = [...pageAssets, ...demos];
  const watchlistQuotes = (savedMarket.data?.data ?? []).map(instrument);
  const savedQuotes = new Map((savedMarket.dataUpdatedAt >= market.dataUpdatedAt ? [...pageAssets, ...watchlistQuotes] : [...watchlistQuotes, ...pageAssets]).map((asset) => [asset.id, asset]));
  const savedAssets = watchlist.map((id): MarketInstrument => savedQuotes.get(id) ?? demos.find((asset) => asset.id === id) ?? { id, symbol: id.slice(7).toUpperCase(), name: id.slice(7), assetClass: "crypto", venue: "Cotação indisponível", currency: "USD", price: null, change: null });
  const filteredAssets = (view === "watchlist" ? savedAssets : assets).filter((asset) => normalize(`${asset.name} ${asset.symbol} ${asset.venue}`).includes(normalize(query)) && (assetClass === "all" || asset.assetClass === assetClass)).sort((a, b) => {
    if (sort === "name") return a.name.localeCompare(b.name, "pt-BR");
    if (sort !== "default") { if (a.change === null) return b.change === null ? 0 : 1; if (b.change === null) return -1; return sort === "gainers" ? b.change - a.change : a.change - b.change; }
    return 0;
  });
  function toggle(id: string) {
    const exists = watchlist.includes(id);
    if (!exists && watchlist.length >= 100) { setMessage("Sua watchlist pode acompanhar até 100 ativos. Remova um para adicionar outro."); return; }
    const next = exists ? watchlist.filter((item) => item !== id) : [...watchlist, id];
    setWatchlist(next);
    try { localStorage.setItem(STORAGE_KEY, JSON.stringify(next)); setStorageError(false); } catch { setStorageError(true); }
    const name = [...assets, ...savedAssets].find((asset) => asset.id === id)?.symbol ?? id;
    setMessage(`${name} ${exists ? "removido da" : "adicionado à"} watchlist.`);
  }
  function explore() { setView("all"); setAssetClass("all"); setQuery(""); searchRef.current?.focus(); }
  const select = (id: string) => setSelected(id.replace(/^crypto:/, ""));
  const cryptoVisible = assetClass === "all" || assetClass === "crypto";
  const activeQuery = view === "watchlist" ? savedMarket : market;
  const loading = !ready || (cryptoVisible && activeQuery.isPending && (view === "all" || cryptoIds.length > 0));

  return <div className="flex min-w-0 flex-col gap-6">
    <header className="flex flex-wrap items-start justify-between gap-4">
      <div><h1 className="text-headline-lg font-semibold tracking-tight">Markets</h1><p className="mt-2 text-body-base text-on-surface-variant">Do Bitcoin à bolsa. Seus mercados em um só lugar.</p></div>
      <span className="rounded-full border border-primary/30 bg-primary/10 px-3 py-1.5 text-xs text-primary">Cripto via CoinGecko</span>
    </header>

    <section aria-labelledby="trending-title">
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2"><h2 id="trending-title" className="text-title-md font-semibold">Em alta nas buscas</h2><a href="https://www.coingecko.com/" target="_blank" rel="noreferrer" className="text-xs text-on-surface-variant underline">Dados por CoinGecko</a></div>
      {trending.isPending && <p role="status" className="py-5 text-body-sm text-on-surface-variant">Carregando tendências…</p>}
      {trending.isError && <p className="text-body-sm text-on-surface-variant">Tendências indisponíveis. <button type="button" onClick={() => void trending.refetch()} className={`text-primary ${FOCUS}`}>Tentar novamente</button></p>}
      <div className="grid gap-4 sm:grid-cols-2 2xl:grid-cols-4">{trending.data?.slice(0, 4).map((coin, index) => <button type="button" key={coin.id} onClick={() => setSelected(coin.id)} className={`flex items-center gap-3 rounded-3xl border border-outline-variant/30 bg-surface-container p-5 text-left hover:bg-surface-container-high ${FOCUS}`}><AssetBadge asset={instrument(coin)} /><span className="min-w-0 flex-1"><span className="block truncate font-medium">{coin.name}</span><span className="text-body-sm text-on-surface-variant">{coin.symbol.toUpperCase()}</span></span><span className="text-primary">#{index + 1}</span></button>)}</div>
      {trending.data?.length === 0 && <p className="text-body-sm text-on-surface-variant">Nenhuma tendência disponível no momento.</p>}
    </section>

    <div className="grid min-w-0 items-start gap-6 xl:grid-cols-[minmax(0,1fr)_300px]">
      <section aria-labelledby="explore-title" className="min-w-0 overflow-hidden rounded-3xl border border-outline-variant/30 bg-surface-container">
        <div className="p-5 sm:p-6">
          <div className="flex flex-wrap items-center justify-between gap-3"><h2 id="explore-title" className="text-headline-md font-semibold">Explore os mercados</h2><button type="button" disabled={activeQuery.isFetching || (view === "watchlist" && cryptoIds.length === 0)} onClick={() => void activeQuery.refetch()} className={`rounded-full border border-outline-variant px-3 py-2 text-body-sm disabled:opacity-50 ${FOCUS}`}>{activeQuery.isFetching ? "Atualizando…" : "Atualizar"}</button></div>
          <p className="mt-2 text-xs leading-relaxed text-on-surface-variant">Cripto: cotações em USD, atualização a cada minuto. Outros mercados: demonstração.</p>
          <div role="group" aria-label="Exibição de ativos" className="mt-5 flex w-fit flex-wrap gap-1 rounded-full bg-surface p-1">{([['all', 'Todos os ativos'], ['watchlist', 'Minha watchlist']] as const).map(([value, label]) => <button key={value} type="button" aria-pressed={view === value} onClick={() => setView(value)} className={`rounded-full px-4 py-2 text-body-sm font-medium ${FOCUS} ${view === value ? "bg-primary/15 text-primary" : "text-on-surface-variant hover:bg-surface-container-high"}`}>{label}{value === "watchlist" && <span className="ml-2 text-xs">{watchlist.length}</span>}</button>)}</div>
          <div className="mt-5 flex flex-wrap gap-3">
            <label className="flex min-w-0 flex-1 basis-56 items-center rounded-full border border-outline-variant/50 bg-surface px-4 focus-within:border-primary"><span className="sr-only">Buscar ativos nesta página</span><input ref={searchRef} value={query} onChange={(event) => setQuery(event.target.value)} placeholder={view === "watchlist" ? "Buscar na watchlist" : "Buscar nesta página"} className="w-full min-w-0 bg-transparent py-3 text-body-sm outline-none placeholder:text-on-surface-variant/60" /></label>
            <select aria-label="Ordenar ativos desta página" value={sort} onChange={(event) => setSort(event.target.value)} className={`rounded-full border border-outline-variant/50 bg-surface px-4 py-3 text-body-sm ${FOCUS}`}><option value="default">Ordem padrão</option><option value="gainers">Maiores altas</option><option value="losers">Maiores baixas</option><option value="name">Nome A–Z</option></select>
          </div>
          <div role="group" aria-label="Classe de ativo" className="mt-4 flex flex-wrap gap-2">{([["all", "Todos"], ...classes] as const).map(([value, label]) => <button key={value} type="button" aria-pressed={assetClass === value} onClick={() => setAssetClass(value)} className={`rounded-full border px-3 py-2 text-body-sm ${FOCUS} ${assetClass === value ? "border-primary/40 bg-primary/10 text-primary" : "border-outline-variant/40 text-on-surface-variant hover:bg-surface-container-high"}`}>{label}</button>)}</div>
        </div>
        {cryptoVisible && activeQuery.isError && <div role="alert" className="mx-6 mb-5 rounded-2xl border border-error/30 bg-error/5 p-4 text-body-sm"><p>{activeQuery.error.message}</p>{activeQuery.data && <p className="mt-1">Exibindo a última resposta recebida; as cotações podem estar desatualizadas.</p>}<button type="button" onClick={() => void activeQuery.refetch()} className={`mt-2 text-primary ${FOCUS}`}>Tentar novamente</button></div>}
        {loading ? <p role="status" className="p-8 text-body-sm text-on-surface-variant">Carregando ativos…</p> : filteredAssets.length ? <MarketTable assets={filteredAssets} watchlist={watchlist} onToggle={toggle} onSelect={select} /> : <div className="px-6 py-12 text-center"><h3 className="font-semibold">{activeQuery.isError && cryptoVisible ? "Cotações indisponíveis" : "Nenhum ativo encontrado"}</h3><p className="mt-2 text-body-sm text-on-surface-variant">Ajuste os filtros, explore outra página ou adicione ativos à watchlist.</p><button type="button" onClick={explore} className={`mt-5 rounded-full bg-primary px-5 py-2.5 text-body-sm font-medium text-on-primary ${FOCUS}`}>Explorar todos os ativos</button></div>}
        <footer className="border-t border-outline-variant/30 px-6 py-4 text-xs text-on-surface-variant">
          <p aria-live="polite">{filteredAssets.length} ativos nesta seleção. Busca e ordenação se aplicam à página atual.</p>
          {view === "all" && cryptoVisible && <div className="mt-4 flex items-center justify-between gap-2"><button type="button" disabled={page === 1 || market.isFetching} onClick={() => setPage((value) => value - 1)} className={`rounded-full border border-outline-variant px-4 py-2 disabled:opacity-40 ${FOCUS}`}>Anterior</button><span>Página {page}</span><button type="button" disabled={!market.data?.hasMore || market.isFetching || page >= 10000} onClick={() => setPage((value) => value + 1)} className={`rounded-full border border-outline-variant px-4 py-2 disabled:opacity-40 ${FOCUS}`}>Próxima</button></div>}
        </footer>
      </section>

      <aside aria-labelledby="watchlist-title" className="rounded-3xl border border-outline-variant/30 bg-surface-container p-5">
        <div className="flex items-center justify-between"><h2 id="watchlist-title" className="text-title-md font-semibold">Minha watchlist</h2><span className="rounded-full bg-primary/10 px-2.5 py-1 text-xs text-primary">{watchlist.length}</span></div>
        <p className="mt-2 text-body-sm text-on-surface-variant">Seus mercados, lado a lado.</p>
        {savedMarket.isError && <p role="status" className="mt-3 text-xs text-error">Não foi possível atualizar sua watchlist. Cotações anteriores podem estar desatualizadas.</p>}
        <div className="mt-5 max-h-[560px] divide-y divide-outline-variant/30 overflow-y-auto">{ready && savedAssets.map((asset) => <div key={asset.id} className="flex items-center gap-2 py-4"><AssetBadge asset={asset} /><div className="min-w-0 flex-1"><p className="truncate text-body-sm font-medium" title={asset.name}>{asset.symbol}</p><p className="mt-1 text-xs text-on-surface-variant">{formatMarketPrice(asset)}{asset.assetClass !== "crypto" && " · Demo"}</p></div><span className={`text-xs tabular-nums ${changeToneClass(asset.change ?? 0)}`}>{asset.change === null ? "—" : formatPercent(asset.change)}</span><WatchlistButton name={asset.name} selected onToggle={() => toggle(asset.id)} /></div>)}{ready && !savedAssets.length && <p className="py-6 text-body-sm text-on-surface-variant">Marque a estrela de qualquer ativo para adicioná-lo aqui.</p>}</div>
        <button type="button" onClick={explore} className={`mt-4 w-full rounded-full border border-primary/40 px-4 py-3 text-body-sm font-medium text-primary hover:bg-primary/10 ${FOCUS}`}>+ Adicionar ativos</button>
        <p className="mt-4 text-xs leading-relaxed text-on-surface-variant">{storageError ? "Armazenamento indisponível. As escolhas valem apenas nesta visita." : "Salva neste navegador, até 100 ativos. Sem sincronização com sua conta."}</p>
      </aside>
    </div>
    <p role="status" className="text-body-sm text-on-surface-variant">{message}</p>
    <p className="text-xs text-on-surface-variant">Cripto usa dados CoinGecko com cache de 60s. Ações, ETFs, índices, câmbio e commodities permanecem simulados. Sem execução de ordens.</p>
    {selected && <MarketDetails key={selected} id={selected} onClose={() => setSelected(null)} />}
  </div>;
}
