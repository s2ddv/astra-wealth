"use client";

import { changeToneClass, formatPercent } from "@/lib/format";
import { ASSET_CLASS_LABELS, formatMarketPrice, type MarketInstrument } from "@/types/markets";
import { WatchlistButton } from "./WatchlistButton";

export function AssetBadge({ asset }: { asset: MarketInstrument }) {
  return <span aria-hidden="true" className={`flex h-10 w-10 shrink-0 items-center justify-center rounded-full text-xs font-semibold ${asset.assetClass === "crypto" ? "bg-primary/15 text-primary" : "bg-secondary/10 text-secondary"}`}>{asset.symbol.slice(0, 2)}</span>;
}

export function MarketTable({ assets, watchlist, onToggle }: { assets: MarketInstrument[]; watchlist: string[]; onToggle: (id: string) => void }) {
  return (
    <div className="overflow-x-auto" tabIndex={0} role="region" aria-label="Tabela de ativos, role horizontalmente para ver todas as colunas">
      <table className="w-full min-w-[620px] text-left text-body-sm">
        <caption className="sr-only">Ativos de demonstração. Variação em 24h para cripto e por sessão para os demais mercados.</caption>
        <thead className="border-y border-outline-variant/30 bg-surface/40 text-on-surface-variant">
          <tr><th scope="col" className="py-4 pl-4"><span className="sr-only">Watchlist</span></th><th scope="col" className="p-4 font-medium">Ativo</th><th scope="col" className="p-4 font-medium">Mercado</th><th scope="col" className="p-4 text-right font-medium">Preço</th><th scope="col" className="p-4 pr-6 text-right font-medium">Variação</th></tr>
        </thead>
        <tbody className="divide-y divide-outline-variant/20">
          {assets.map((asset) => (
            <tr key={asset.id} className="transition-colors hover:bg-surface-container-high/50">
              <td className="py-3 pl-4"><WatchlistButton name={asset.name} selected={watchlist.includes(asset.id)} onToggle={() => onToggle(asset.id)} /></td>
              <th scope="row" className="p-4 font-medium"><div className="flex items-center gap-3"><AssetBadge asset={asset} /><div>{asset.symbol}<p className="mt-1 font-normal text-on-surface-variant">{asset.name}</p></div></div></th>
              <td className="p-4"><span className="rounded-full bg-surface-container-high px-2.5 py-1 text-xs">{ASSET_CLASS_LABELS[asset.assetClass]}</span><p className="mt-2 text-xs text-on-surface-variant">{asset.venue}</p></td>
              <td className="p-4 text-right tabular-nums"><span className="whitespace-nowrap">{formatMarketPrice(asset)}</span><p className="mt-1 text-xs text-on-surface-variant">{asset.assetClass === "indices" ? "Pontos" : asset.unit ?? asset.currency}</p></td>
              <td className="p-4 pr-6 text-right"><span className={`whitespace-nowrap font-medium tabular-nums ${changeToneClass(asset.change)}`}>{formatPercent(asset.change)}</span><p className="mt-1 text-xs text-on-surface-variant">{asset.assetClass === "crypto" ? "24h" : "Sessão"}</p></td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
