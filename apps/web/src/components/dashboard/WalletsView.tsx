"use client";

import { useState } from "react";
import {
  CHAIN_LABELS,
  truncateAddress,
} from "@/lib/format";
import { useWallets, useWalletAssets, useWalletMutations } from "@/hooks/useWallets";
const dollars = (value: string | null | undefined) => value == null ? "Indisponível" : `US$ ${value}`;
import { AddWalletModal } from "./AddWalletModal";

export function WalletsView() {
  const walletsQuery = useWallets();
  const wallets = walletsQuery.data ?? [];
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const selected = wallets.find((wallet) => wallet.id === selectedId) ?? wallets[0];
  const assetsQuery = useWalletAssets(selected?.id);
  const { sync, remove } = useWalletMutations();
  const snapshot = assetsQuery.data;
  return (
    <section className="flex flex-col gap-8">
      <header className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="text-label-caps uppercase text-on-surface-variant">Accounts</p>
          <h1 className="text-headline-lg font-bold text-on-surface">Wallets</h1>
          <p className="mt-1 text-body-base text-on-surface-variant">
            Carteiras monitoradas com saldos e cotações dos provedores.
          </p>
        </div>
        <button
          type="button"
          onClick={() => setIsModalOpen(true)}
          className="flex items-center gap-2 rounded-full bg-primary px-5 py-3 font-medium text-on-primary hover:bg-primary/90"
        >
          <span className="material-symbols-outlined text-[20px]">add</span>
          Adicionar carteira
        </button>
      </header>

      {walletsQuery.isPending && <p role="status">Carregando carteiras…</p>}
      {walletsQuery.error && <p role="alert">{walletsQuery.error.message} <button onClick={() => void walletsQuery.refetch()}>Tentar novamente</button></p>}
      {(sync.error || remove.error) && <p role="alert">{(sync.error || remove.error)?.message}</p>}
      <div className="grid gap-6 xl:grid-cols-2">
        <ul className="flex flex-col gap-3">
          {wallets.map((wallet) => {
            const isActive = wallet.id === selected?.id;
            return (
              <li key={wallet.id}>
                <button
                  type="button"
                  onClick={() => setSelectedId(wallet.id)}
                  aria-pressed={isActive}
                  className={`flex w-full items-center justify-between gap-4 rounded-3xl border px-5 py-4 text-left transition-colors ${
                    isActive
                      ? "border-primary/40 bg-primary/10"
                      : "border-outline-variant/30 bg-surface-container hover:border-outline-variant/60"
                  }`}
                >
                  <div className="min-w-0">
                    <p className="text-title-md font-semibold text-on-surface">
                      {wallet.nickname || "Carteira"}
                    </p>
                    <p className="mt-1 break-all font-mono text-body-sm text-on-surface-variant">
                      {truncateAddress(wallet.address)}
                    </p>
                    <p className="mt-2 text-label-caps uppercase text-on-surface-variant">
                      {CHAIN_LABELS[wallet.chain.toLowerCase() as keyof typeof CHAIN_LABELS] ?? wallet.chain}
                    </p>
                  </div>
                  <p className="shrink-0 text-title-md font-semibold text-on-surface">
                    {wallet.id === selected?.id && snapshot ? dollars(snapshot.totalUsd) : "—"}
                  </p>
                </button>
              </li>
            );
          })}
        </ul>

        <div className="min-w-0 rounded-3xl border border-outline-variant/30 bg-surface-container p-6">
          {selected ? (
            <>
              <div className="mb-6">
                <p className="text-label-caps uppercase text-on-surface-variant">
                  Holdings
                </p>
                <h2 className="text-headline-md font-bold text-on-surface">
                  {selected.nickname || "Carteira"}
                </h2>
                <p className="mt-1 break-all font-mono text-body-sm text-on-surface-variant">
                  {selected.address}
                </p>
              </div>
              <div className="mb-4 flex gap-4">
                <button disabled={sync.isPending || assetsQuery.isFetching} onClick={() => sync.mutate(selected.id)} className="text-primary disabled:opacity-50">{sync.isPending ? "Sincronizando…" : "Sincronizar"}</button>
                <button disabled={remove.isPending} onClick={() => { if (window.confirm("Excluir esta carteira?")) remove.mutate(selected.id); }} className="text-error">Excluir carteira</button>
              </div>
              {assetsQuery.isPending && <p role="status">Consultando saldos…</p>}
              {assetsQuery.error && <p role="alert">{assetsQuery.error.message} <button onClick={() => void assetsQuery.refetch()}>Tentar novamente</button></p>}
              {snapshot && <p className="mb-4 text-body-sm">{!snapshot.complete ? "Saldos parciais. Alguns ativos não puderam ser consultados." : !snapshot.valuationComplete ? "Cotação parcial. Alguns ativos estão sem preço." : "Saldos e cotações completos."} Atualizado em {new Date(snapshot.updatedAt).toLocaleString("pt-BR")}.</p>}
              <div className="overflow-x-auto">
              <table className="w-full min-w-[440px] text-left">
                <caption className="sr-only">Ativos de {selected.nickname || "Carteira"}</caption>
                <thead>
                  <tr className="text-label-caps uppercase text-on-surface-variant">
                    <th className="pb-3 font-medium">Ativo</th>
                    <th className="pb-3 font-medium">Qtd</th>
                    <th className="pb-3 font-medium">Preço</th>
                    <th className="pb-3 text-right font-medium">Valor</th>
                  </tr>
                </thead>
                <tbody>
                  {(snapshot?.assets ?? []).map((asset) => (
                    <tr
                      key={asset.isNative ? "native" : asset.contractAddress ?? asset.symbol}
                      className="border-t border-outline-variant/20"
                    >
                      <td className="py-3">
                        <p className="text-body-base font-medium text-on-surface">
                          {asset.name}
                        </p>
                        <p className="text-body-sm text-on-surface-variant">
                          {asset.symbol}

                        </p>
                      </td>
                      <td className="py-3 font-mono text-body-sm text-on-surface">
                        {asset.amount}
                      </td>
                      <td className="py-3 font-mono text-body-sm text-on-surface">
                        {dollars(asset.priceUsd)}
                      </td>
                      <td className="py-3 text-right font-mono text-body-sm text-on-surface">
                        {dollars(asset.valueUsd)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
              </div>
            </>
          ) : (
            <p className="text-body-base text-on-surface-variant">
              {walletsQuery.isError ? "Não foi possível consultar suas carteiras." : walletsQuery.isPending ? "Aguarde o carregamento." : "Nenhuma carteira cadastrada."}
            </p>
          )}
        </div>
      </div>

      <AddWalletModal open={isModalOpen} onClose={() => setIsModalOpen(false)} />
    </section>
  );
}
