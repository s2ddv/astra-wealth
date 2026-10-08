"use client";

import { useEffect, useId, useRef, useState } from "react";
import { CHAIN_LABELS } from "@/lib/format";
import type { AddWalletInput, ChainId } from "@/types/dashboard";

const CHAINS: ChainId[] = ["ethereum", "polygon", "arbitrum", "base"];

import { useWalletMutations } from "@/hooks/useWallets";
import type { WalletChain } from "../../../../../packages/shared/src/generated/rust/WalletDto";

interface AddWalletModalProps {
  open: boolean;
  onClose: () => void;
}

const EMPTY_FORM: AddWalletInput = {
  nickname: "",
  address: "",
  chain: "ethereum",
};

export function AddWalletModal({ open, onClose }: AddWalletModalProps) {
  const titleId = useId();
  const { add } = useWalletMutations();
  const dialogRef = useRef<HTMLDialogElement>(null);
  const [form, setForm] = useState<AddWalletInput>(EMPTY_FORM);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (open) {
      setForm(EMPTY_FORM);
      dialog?.showModal();
    } else {
      dialog?.close();
    }
  }, [open]);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (add.isPending) return;
    try {
      await add.mutateAsync({ ...form, nickname: form.nickname.trim(), address: form.address.trim(), chain: form.chain.toUpperCase() as WalletChain });
      onClose();
    } catch { /* The mutation error remains visible inside the dialog. */ }
  }

  return (
    <dialog
      ref={dialogRef}
      aria-labelledby={titleId}
      onCancel={onClose}
      onClick={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
      className="m-auto max-h-[90vh] w-full max-w-md overflow-y-auto rounded-3xl border border-outline-variant/40 bg-surface-container-high p-0 text-on-surface shadow-2xl backdrop:bg-background/70 backdrop:backdrop-blur-sm"
    >
      <div className="p-6">
        <div className="mb-6 flex items-start justify-between gap-4">
          <div>
            <h2 id={titleId} className="text-headline-md font-bold text-on-surface">
              Adicionar carteira
            </h2>
            <p className="mt-1 text-body-sm text-on-surface-variant">
              Cadastre um endereço público para acompanhar seus saldos.
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Fechar"
            className="flex h-10 w-10 items-center justify-center rounded-full text-on-surface-variant hover:bg-surface-variant hover:text-on-surface"
          >
            <span className="material-symbols-outlined">close</span>
          </button>
        </div>

        <form className="flex flex-col gap-4" onSubmit={handleSubmit}>
          <label className="flex flex-col gap-2">
            <span className="text-label-caps uppercase text-on-surface-variant">
              Nickname
            </span>
            <input
              autoFocus
              required
              value={form.nickname}
              onChange={(event) =>
                setForm((current) => ({ ...current, nickname: event.target.value }))
              }
              placeholder="Ex.: Tesouro ETH"
              className="rounded-2xl border border-outline-variant/50 bg-surface px-4 py-3 text-body-base text-on-surface outline-none placeholder:text-on-surface-variant/50 focus:border-primary"
            />
          </label>

          <label className="flex flex-col gap-2">
            <span className="text-label-caps uppercase text-on-surface-variant">
              Endereço
            </span>
            <input
              required
              value={form.address}
              onChange={(event) =>
                setForm((current) => ({ ...current, address: event.target.value }))
              }
              placeholder="0x…"
              className="rounded-2xl border border-outline-variant/50 bg-surface px-4 py-3 font-mono text-body-sm text-on-surface outline-none placeholder:text-on-surface-variant/50 focus:border-primary"
            />
          </label>

          <label className="flex flex-col gap-2">
            <span className="text-label-caps uppercase text-on-surface-variant">
              Chain
            </span>
            <select
              value={form.chain}
              onChange={(event) =>
                setForm((current) => ({
                  ...current,
                  chain: event.target.value as ChainId,
                }))
              }
              className="rounded-2xl border border-outline-variant/50 bg-surface px-4 py-3 text-body-base text-on-surface outline-none focus:border-primary"
            >
              {CHAINS.map((chain) => (
                <option key={chain} value={chain}>
                  {CHAIN_LABELS[chain]}
                </option>
              ))}
            </select>
          </label>

          {add.error && <p role="alert" className="text-error">{add.error.message}</p>}
          <div className="mt-2 flex justify-end gap-3">
            <button
              type="button"
              onClick={onClose}
              className="rounded-full px-4 py-2 text-body-sm text-on-surface-variant hover:bg-surface-variant"
            >
              Cancelar
            </button>
            <button
              type="submit"
              disabled={add.isPending}
              className="rounded-full bg-primary px-5 py-2 text-body-sm font-medium text-on-primary hover:bg-primary/90"
            >
              {add.isPending ? "Salvando…" : "Adicionar carteira"}
            </button>
          </div>
        </form>
      </div>
    </dialog>
  );
}
