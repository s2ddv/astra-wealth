"use client";

import Link from "next/link";
import { CHAIN_LABELS, formatUsd, truncateAddress } from "@/lib/format";
import type { PortfolioChain, SummaryWallet } from "@/types/portfolio";

// Chain symbols are separate from the Material Symbols interface icons.
const CHAIN_SYMBOLS: Record<PortfolioChain, string> = {
  ethereum: "Ξ", polygon: "POL", arbitrum: "ARB", base: "━", solana: "◎", bitcoin: "₿",
};

interface WalletsSummaryProps {
  wallets: SummaryWallet[];
  onAddWallet: () => void;
}

export function WalletsSummary({ wallets, onAddWallet }: WalletsSummaryProps) {
  return (
    <section aria-labelledby="hero-wallets-title" className="min-w-0 rounded-3xl border border-outline-variant/30 bg-surface-container p-5 sm:p-6">
      <header className="mb-5 flex flex-wrap items-center justify-between gap-2">
        <h2 id="hero-wallets-title" className="text-title-md font-semibold">Carteiras</h2>
        <span className="text-body-sm text-on-surface-variant">{wallets.length} {wallets.length === 1 ? "conectada" : "conectadas"}</span>
      </header>
      {wallets.length ? (
        <ul className="space-y-3">
          {wallets.map((wallet) => {
            const name = wallet.nickname?.trim() || truncateAddress(wallet.address);
            const chain = wallet.chain === "bitcoin" ? "Bitcoin" : CHAIN_LABELS[wallet.chain];
            return (
              <li key={wallet.id} className="flex items-center gap-3 rounded-2xl border border-outline-variant/20 bg-surface-container-high/60 p-3">
                <span title={chain} aria-label={chain} className={`flex size-10 shrink-0 items-center justify-center rounded-full bg-primary/10 font-semibold text-primary ${wallet.chain === "polygon" || wallet.chain === "arbitrum" ? "text-xs" : "text-2xl"}`}>
                  {CHAIN_SYMBOLS[wallet.chain]}
                </span>
                <div className="min-w-0 flex-1">
                  <p title={name} className="truncate text-body-base font-medium">{name}</p>
                  <p className="mt-0.5 text-body-sm tabular-nums text-on-surface-variant">{formatUsd(wallet.valueUsd)}</p>
                  <span className={`mt-2 inline-flex items-center gap-1.5 rounded-full px-2 py-0.5 text-[11px] font-medium ${wallet.status === "synced" ? "bg-secondary/10 text-secondary" : "bg-[#e14a65]/10 text-[#e14a65]"}`}>
                    <span aria-hidden="true" className="size-1.5 rounded-full bg-current" />
                    {wallet.status === "synced" ? "Sincronizado" : "Erro"}
                  </span>
                </div>
                <details className="group relative shrink-0">
                  <summary aria-label={`Opções de ${name}`} className="flex size-9 cursor-pointer list-none items-center justify-center rounded-full text-on-surface-variant hover:bg-surface-container-highest focus-visible:outline-2 focus-visible:outline-primary [&::-webkit-details-marker]:hidden">
                    <span aria-hidden="true" className="material-symbols-outlined">more_vert</span>
                  </summary>
                  <div className="absolute right-0 top-10 z-10 w-44 rounded-xl border border-outline-variant bg-surface-container-high p-1 shadow-xl">
                    <Link href="/dashboard/wallets" className="block rounded-lg px-3 py-2 text-body-sm hover:bg-primary/10 focus-visible:outline-primary">Ver carteiras</Link>
                  </div>
                </details>
              </li>
            );
          })}
        </ul>
      ) : (
        <div className="flex min-h-64 flex-col items-center justify-center text-center">
          <span aria-hidden="true" className="material-symbols-outlined mb-4 text-primary">account_balance_wallet</span>
          <p className="font-medium">Nenhuma carteira conectada</p>
          <p className="mt-2 text-body-sm text-on-surface-variant">Adicione sua primeira carteira para acompanhar seu patrimônio.</p>
          <button type="button" onClick={onAddWallet} className="mt-5 rounded-full bg-primary px-4 py-2 text-body-sm font-medium text-on-primary focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-primary">Adicionar carteira</button>
        </div>
      )}
    </section>
  );
}
