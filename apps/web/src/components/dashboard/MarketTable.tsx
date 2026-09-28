"use client";

import { useState } from "react";
import { changeToneClass, formatPercent } from "@/lib/format";
import { ASSET_CLASS_LABELS, formatMarketPrice, type MarketInstrument } from "@/types/markets";
import { WatchlistButton } from "./WatchlistButton";

export function AssetBadge({ asset }: { asset: MarketInstrument }) {
  const [failedUrl, setFailedUrl] = useState<string | null>(null);
  const hash = [...asset.symbol.toUpperCase()].reduce((value, char) => ((value * 31) + char.charCodeAt(0)) >>> 0, 0);
  const color = `hsl(${hash % 360} 65% 78%)`;
  return <span aria-hidden="true" style={{ color, backgroundColor: `hsl(${hash % 360} 35% 20%)` }} className="flex h-10 w-10 shrink-0 items-center justify-center overflow-hidden rounded-full text-xs font-semibold">
    {asset.image && failedUrl !== asset.image ? <img src={asset.image} alt="" width={40} height={40} referrerPolicy="no-referrer" onError={() => setFailedUrl(asset.image ?? null)} className="h-full w-full object-cover" /> : asset.symbol.slice(0, 2).toUpperCase()}
  </span>;
}

export function MarketTable({ assets, watchlist, onToggle, onSelect }: { assets: MarketInstrument[]; watchlist: string[]; onToggle: (id: string) => void; onSelect: (id: string) => void }) {
  return (
    <div className="overflow-x-auto" tabIndex={0} role="region" aria-label="Tabela de ativos, role horizontalmente para ver todas as colunas">
      <table className="w-full min-w-[620px] text-left text-body-sm">
        <caption className="sr-only">Cripto via CoinGecko; outros ativos de demonstração. Variação em 24h para cripto e por sessão para os demais mercados.</caption>
        <thead className="border-y border-outline-variant/30 bg-surface/40 text-on-surface-variant">
          <tr><th scope="col" className="py-4 pl-4"><span className="sr-only">Watchlist</span></th><th scope="col" className="p-4 font-medium">Ativo</th><th scope="col" className="p-4 font-medium">Mercado</th><th scope="col" className="p-4 text-right font-medium">Preço</th><th scope="col" className="p-4 pr-6 text-right font-medium">Variação</th></tr>
        </thead>
        <tbody className="divide-y divide-outline-variant/20">
          {assets.map((asset) => (
            <tr key={asset.id} className="transition-colors hover:bg-surface-container-high/50">
              <td className="py-3 pl-4"><WatchlistButton name={asset.name} selected={watchlist.includes(asset.id)} onToggle={() => onToggle(asset.id)} /></td>
              <th scope="row" className="p-4 font-medium"><div className="flex items-center gap-3"><AssetBadge asset={asset} /><div>{asset.assetClass === "crypto" ? <button type="button" className="text-primary underline-offset-4 hover:underline focus-visible:outline-2 focus-visible:outline-primary" onClick={() => onSelect(asset.id)} aria-label={`Ver detalhes de ${asset.name}`}>{asset.symbol}</button> : asset.symbol}<p className="mt-1 font-normal text-on-surface-variant">{asset.name}</p></div></div></th>
              <td className="p-4"><span className="rounded-full bg-surface-container-high px-2.5 py-1 text-xs">{ASSET_CLASS_LABELS[asset.assetClass]}</span><p className="mt-2 text-xs text-on-surface-variant">{asset.venue}{asset.assetClass !== "crypto" ? " · Demo" : ""}</p></td>
              <td className="p-4 text-right tabular-nums"><span className="whitespace-nowrap">{formatMarketPrice(asset)}</span><p className="mt-1 text-xs text-on-surface-variant">{asset.assetClass === "indices" ? "Pontos" : asset.unit ?? asset.currency}</p></td>
              <td className="p-4 pr-6 text-right"><span className={`whitespace-nowrap font-medium tabular-nums ${changeToneClass(asset.change ?? 0)}`}>{asset.change === null ? "—" : formatPercent(asset.change)}</span><p className="mt-1 text-xs text-on-surface-variant">{asset.assetClass === "crypto" ? "24h" : "Sessão"}</p></td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
