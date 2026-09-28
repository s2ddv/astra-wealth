"use client";
import { useEffect, useRef } from "react";
import { useMarketCoin } from "@/hooks/useMarkets";
import { formatCompactUsd, formatPercent, formatUsd } from "@/lib/format";

export function MarketDetails({ id, onClose }: { id: string; onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const coin = useMarketCoin(id);
  useEffect(() => { const element = dialog.current; element?.showModal(); return () => element?.close(); }, []);
  return <dialog ref={dialog} onClose={onClose} aria-labelledby="coin-details-title" className="m-auto w-[min(90vw,520px)] rounded-3xl border border-outline-variant bg-surface-container p-6 text-on-surface backdrop:bg-black/60">
    <div className="flex items-center justify-between gap-4"><h2 id="coin-details-title" className="text-headline-md font-semibold">{coin.data?.name ?? id}</h2><button type="button" autoFocus onClick={onClose} className="rounded-full border border-outline-variant px-4 py-2 text-body-sm focus-visible:outline-2 focus-visible:outline-primary">Fechar</button></div>
    {coin.isPending && <p role="status" className="py-8">Carregando detalhes…</p>}
    {coin.isError && <div role="alert" className="py-6"><p>{coin.error.message}</p><button type="button" onClick={() => void coin.refetch()} className="mt-4 text-primary">Tentar novamente</button></div>}
    {coin.data && <><p className="mt-2 text-body-sm text-on-surface-variant">{coin.data.symbol.toUpperCase()} · CoinGecko · USD</p><dl className="mt-6 grid grid-cols-2 gap-5">{[
      ["Preço", coin.data.currentPrice === null ? "Indisponível" : formatUsd(coin.data.currentPrice)],
      ["Variação 24h", coin.data.priceChangePercentage24h === null ? "—" : formatPercent(coin.data.priceChangePercentage24h)],
      ["Market cap", coin.data.marketCap === null ? "—" : formatCompactUsd(coin.data.marketCap)],
      ["Volume 24h", coin.data.volume24h === null ? "—" : formatCompactUsd(coin.data.volume24h)],
    ].map(([label, value]) => <div key={label}><dt className="text-body-sm text-on-surface-variant">{label}</dt><dd className="mt-2 font-semibold tabular-nums">{value}</dd></div>)}</dl><p className="mt-6 text-xs text-on-surface-variant">Última cotação: {coin.data.updatedAt ? new Date(coin.data.updatedAt).toLocaleString("pt-BR") : "não informada pelo provedor"}.</p></>}
  </dialog>;
}
