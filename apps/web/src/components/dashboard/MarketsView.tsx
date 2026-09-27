"use client";

import { useEffect, useRef, useState } from "react";
import { changeToneClass, formatPercent } from "@/lib/format";
import { defaultWatchlist, marketHighlights, mockMarketAssets } from "@/mocks/markets";
import { ASSET_CLASS_LABELS, formatMarketPrice, type AssetClass } from "@/types/markets";
import { AssetBadge, MarketTable } from "./MarketTable";
import { WatchlistButton } from "./WatchlistButton";

const STORAGE_KEY = "zora.markets.watchlist.v1";
const FOCUS = "focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-primary";
const classes = Object.entries(ASSET_CLASS_LABELS) as [AssetClass, string][];
const normalize = (value: string) => value.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase().trim();

export function MarketsView() {
  const [watchlist, setWatchlist] = useState<string[]>(defaultWatchlist);
  const [ready, setReady] = useState(false);
  const [storageError, setStorageError] = useState(false);
  const [query, setQuery] = useState("");
  const [assetClass, setAssetClass] = useState<AssetClass | "all">("all");
  const [view, setView] = useState<"all" | "watchlist">("all");
  const [sort, setSort] = useState("default");
  const [message, setMessage] = useState("");
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved !== null) {
        const ids: unknown = JSON.parse(saved);
        if (!Array.isArray(ids) || !ids.every((id) => typeof id === "string")) throw new Error("Invalid watchlist");
        setWatchlist([...new Set(ids.filter((id) => mockMarketAssets.some((asset) => asset.id === id)))]);
      }
    } catch { setStorageError(true); }
    setReady(true);
  }, []);

  function toggle(id: string) {
    const selected = watchlist.includes(id);
    const next = selected ? watchlist.filter((item) => item !== id) : [...watchlist, id];
    setWatchlist(next);
    try { localStorage.setItem(STORAGE_KEY, JSON.stringify(next)); setStorageError(false); }
    catch { setStorageError(true); }
    const asset = mockMarketAssets.find((item) => item.id === id)!;
    setMessage(`${asset.symbol} ${selected ? "removido da" : "adicionado à"} watchlist.`);
  }

  const savedAssets = mockMarketAssets.filter((asset) => watchlist.includes(asset.id));
  const filteredAssets = mockMarketAssets.filter((asset) => {
    const matchesQuery = normalize(`${asset.name} ${asset.symbol} ${asset.venue}`).includes(normalize(query));
    return matchesQuery && (assetClass === "all" || asset.assetClass === assetClass) && (view === "all" || watchlist.includes(asset.id));
  }).sort((a, b) => sort === "gainers" ? b.change - a.change : sort === "losers" ? a.change - b.change : sort === "name" ? a.name.localeCompare(b.name, "pt-BR") : 0);

  function explore() { setView("all"); setAssetClass("all"); setQuery(""); searchRef.current?.focus(); }

  return (
    <div className="flex min-w-0 flex-col gap-6">
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div><h1 className="text-headline-lg font-semibold tracking-tight">Markets</h1><p className="mt-2 text-body-base text-on-surface-variant">Do Bitcoin à bolsa. Seu próximo ativo começa aqui.</p></div>
        <span className="rounded-full border border-outline-variant/40 px-3 py-1.5 text-xs text-on-surface-variant">Cotações de demonstração</span>
      </header>

      <section aria-label="Panorama dos mercados" className="grid grid-cols-1 gap-4 min-[680px]:grid-cols-2 2xl:grid-cols-4">
        {marketHighlights.map((id) => {
          const asset = mockMarketAssets.find((item) => item.id === id)!;
          return <article key={id} className="rounded-3xl border border-outline-variant/30 bg-surface-container p-5">
            <div className="flex items-center justify-between gap-3"><p className="text-body-sm text-on-surface-variant">{asset.name}</p><span className="text-xs text-primary">{asset.symbol}</span></div>
            <p className="mt-5 text-2xl font-semibold tracking-tight tabular-nums">{formatMarketPrice(asset)}</p>
            <div className="mt-2 flex items-center gap-2 text-body-sm"><span className={`font-medium ${changeToneClass(asset.change)}`}>{formatPercent(asset.change)}</span><span className="text-on-surface-variant">{asset.assetClass === "crypto" ? "em 24h" : "na sessão"}</span></div>
          </article>;
        })}
      </section>

      <div className="grid min-w-0 items-start gap-6 xl:grid-cols-[minmax(0,1fr)_300px]">
        <section aria-labelledby="explore-title" className="min-w-0 overflow-hidden rounded-3xl border border-outline-variant/30 bg-surface-container">
          <div className="p-5 sm:p-6">
            <div className="flex flex-wrap items-center justify-between gap-3"><h2 id="explore-title" className="text-headline-md font-semibold">Explore os mercados</h2><span className="text-body-sm text-on-surface-variant">{mockMarketAssets.length} ativos · 6 mercados</span></div>
            <div role="group" aria-label="Exibição de ativos" className="mt-5 flex w-fit flex-wrap gap-1 rounded-full bg-surface p-1">
              {([['all', 'Todos os ativos'], ['watchlist', 'Minha watchlist']] as const).map(([value, label]) => <button key={value} type="button" aria-pressed={view === value} onClick={() => setView(value)} className={`rounded-full px-4 py-2 text-body-sm font-medium ${FOCUS} ${view === value ? "bg-primary/15 text-primary" : "text-on-surface-variant hover:bg-surface-container-high"}`}>{label}{value === "watchlist" && <span className="ml-2 text-xs">{watchlist.length}</span>}</button>)}
            </div>
            <div className="mt-5 flex flex-wrap gap-3">
              <label className="flex min-w-0 flex-1 basis-56 items-center gap-2 rounded-full border border-outline-variant/50 bg-surface px-4 focus-within:border-primary">
                <svg aria-hidden="true" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="shrink-0 text-on-surface-variant"><circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 5 5"/></svg>
                <span className="sr-only">Buscar ativos</span><input ref={searchRef} value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Buscar nome, símbolo ou bolsa" className="w-full min-w-0 bg-transparent py-3 text-body-sm outline-none placeholder:text-on-surface-variant/60" />
              </label>
              <select aria-label="Ordenar ativos" value={sort} onChange={(event) => setSort(event.target.value)} className={`rounded-full border border-outline-variant/50 bg-surface px-4 py-3 text-body-sm ${FOCUS}`}><option value="default">Ordem padrão</option><option value="gainers">Maiores altas</option><option value="losers">Maiores baixas</option><option value="name">Nome A–Z</option></select>
            </div>
            <div role="group" aria-label="Classe de ativo" className="mt-4 flex flex-wrap gap-2">
              {([["all", "Todos"], ...classes] as const).map(([value, label]) => <button key={value} type="button" aria-pressed={assetClass === value} onClick={() => setAssetClass(value)} className={`rounded-full border px-3 py-2 text-body-sm transition-colors ${FOCUS} ${assetClass === value ? "border-primary/40 bg-primary/10 text-primary" : "border-outline-variant/40 text-on-surface-variant hover:bg-surface-container-high"}`}>{label}</button>)}
            </div>
          </div>
          {!ready ? <p role="status" className="p-8 text-body-sm text-on-surface-variant">Carregando sua watchlist…</p> : filteredAssets.length ? <MarketTable assets={filteredAssets} watchlist={watchlist} onToggle={toggle} /> : <div className="px-6 py-14 text-center"><h3 className="font-semibold">{view === "watchlist" && !watchlist.length ? "Sua watchlist começa com uma escolha" : "Nenhum ativo encontrado"}</h3><p className="mt-2 text-body-sm text-on-surface-variant">{view === "watchlist" && !watchlist.length ? "Explore os mercados e toque na estrela para acompanhar um ativo." : "Tente outro símbolo ou ajuste os filtros."}</p><button type="button" onClick={explore} className={`mt-5 rounded-full bg-primary px-5 py-2.5 text-body-sm font-medium text-on-primary ${FOCUS}`}>Explorar todos os ativos</button></div>}
          <footer className="border-t border-outline-variant/30 px-6 py-4 text-xs text-on-surface-variant"><p aria-live="polite">{filteredAssets.length} de {mockMarketAssets.length} ativos · Preços na moeda de cada mercado.</p><p className="mt-1">Cripto: variação em 24h. Demais mercados: variação por sessão.</p></footer>
        </section>

        <aside aria-labelledby="watchlist-title" className="rounded-3xl border border-outline-variant/30 bg-surface-container p-5">
          <div className="flex items-center justify-between"><h2 id="watchlist-title" className="text-title-md font-semibold">Minha watchlist</h2><span className="rounded-full bg-primary/10 px-2.5 py-1 text-xs text-primary">{watchlist.length}</span></div>
          <p className="mt-2 text-body-sm text-on-surface-variant">Seus mercados, lado a lado.</p>
          <div className="mt-5 divide-y divide-outline-variant/30">
            {ready && savedAssets.map((asset) => <div key={asset.id} className="flex items-center gap-2 py-4"><AssetBadge asset={asset} /><div className="min-w-0 flex-1"><p className="text-body-sm font-medium">{asset.symbol}</p><p className="mt-1 text-xs text-on-surface-variant">{formatMarketPrice(asset)}</p></div><span className={`text-xs tabular-nums ${changeToneClass(asset.change)}`}>{formatPercent(asset.change)}</span><WatchlistButton name={asset.name} selected onToggle={() => toggle(asset.id)} /></div>)}
            {ready && !savedAssets.length && <p className="py-6 text-body-sm text-on-surface-variant">Marque a estrela de qualquer ativo para adicioná-lo aqui.</p>}
          </div>
          <button type="button" onClick={explore} className={`mt-4 w-full rounded-full border border-primary/40 px-4 py-3 text-body-sm font-medium text-primary hover:bg-primary/10 ${FOCUS}`}>+ Adicionar ativos</button>
          <p className="mt-4 text-xs leading-relaxed text-on-surface-variant">{storageError ? "Não foi possível acessar o armazenamento. Suas escolhas valem apenas nesta visita." : "Salva neste navegador. Sem sincronização com sua conta."}</p>
          <div className="mt-5 border-t border-outline-variant/30 pt-5"><p className="text-body-sm font-medium">Um olhar além do cripto</p><p className="mt-2 text-xs leading-relaxed text-on-surface-variant">Acompanhe ações brasileiras e globais, ETFs, índices, moedas e commodities na mesma lista.</p></div>
        </aside>
      </div>
      <p role="status" className="sr-only">{message}</p>
      <p className="text-xs text-on-surface-variant">Catálogo demonstrativo com cotações simuladas. Não representa preços atuais nem oferece execução de ordens.</p>
    </div>
  );
}
