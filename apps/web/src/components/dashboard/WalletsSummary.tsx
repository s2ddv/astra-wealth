"use client";
import Link from "next/link";
import type { FinancialAccountDTO } from "@astra-wealth/shared/accounts";
import { ACCOUNT_ICONS, money, syncLabel } from "./AccountPresentation";

export function WalletsSummary({
  accounts,
}: {
  accounts: FinancialAccountDTO[];
}) {
  return (
    <section
      aria-labelledby="hero-accounts-title"
      className="min-w-0 rounded-3xl border border-outline-variant/30 bg-surface-container p-5 sm:p-6"
    >
      <header className="mb-5 flex items-center justify-between gap-2">
        <h2 id="hero-accounts-title" className="text-title-md font-semibold">
          Contas
        </h2>
        <span className="text-body-sm text-on-surface-variant">
          {accounts.length} contas
        </span>
      </header>
      <ul className="space-y-3">
        {accounts.map((account) => (
          <li key={account.id}>
            <Link
              href={`/dashboard/accounts/${account.id}`}
              className="flex items-center gap-3 rounded-2xl border border-outline-variant/20 bg-surface-container-high/60 p-3 hover:border-primary/60 focus-visible:outline-primary"
            >
              <span
                aria-hidden="true"
                className="material-symbols-outlined rounded-full bg-primary/10 p-2 text-primary"
              >
                {ACCOUNT_ICONS[account.kind]}
              </span>
              <div className="min-w-0">
                <p className="truncate text-body-base font-medium">
                  {account.name}
                </p>
                <p className="mt-1 text-body-sm tabular-nums">
                  {money(account.currentValue)}
                </p>
                <p
                  className={`mt-2 text-xs ${account.syncStatus === "NEEDS_REAUTH" ? "text-error" : "text-on-surface-variant"}`}
                >
                  {syncLabel(account)}
                </p>
              </div>
            </Link>
          </li>
        ))}
      </ul>
      <Link
        href="/dashboard/accounts"
        className="mt-5 inline-block text-body-sm text-primary"
      >
        Ver todas as contas
      </Link>
    </section>
  );
}
