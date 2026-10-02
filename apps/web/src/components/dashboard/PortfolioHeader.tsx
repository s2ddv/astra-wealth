"use client";

import { useEffect, useState } from "react";
import { formatPercent } from "@/lib/format";
import { mockPortfolio } from "@/mocks/portfolio";
import { mockUserProfile } from "@/mocks/settings";
import type { PortfolioPeriod } from "@/types/portfolio";
import Link from "next/link";
import { useAccounts } from "./AccountsProvider";
import { ContributionsKpis } from "./AccountPresentation";
import { summarizeAccounts } from "@/mocks/accounts";
import { WalletsSummary } from "./WalletsSummary";

const PERIODS: PortfolioPeriod[] = ["24h", "7d", "30d", "1A", "All"];
const FOCUS = "focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-primary";

interface PortfolioHeaderProps {
  userName?: string;
  onPeriodChange?: (period: PortfolioPeriod) => void;
}

export function PortfolioHeader({ userName = mockUserProfile.displayName, onPeriodChange }: PortfolioHeaderProps) {
  const [greeting, setGreeting] = useState("Olá");
  const [period, setPeriod] = useState<PortfolioPeriod>("30d");
  const [currency, setCurrency] = useState<"USD" | "BRL">("USD");
  const { accounts, contributions } = useAccounts();
  const summary = summarizeAccounts(accounts, contributions);
  const [refreshMessage, setRefreshMessage] = useState("");

  useEffect(() => {
    function updateGreeting() {
      const hour = new Date().getHours();
      setGreeting(hour < 12 ? "Bom dia" : hour < 18 ? "Boa tarde" : "Boa noite");
    }
    updateGreeting();
    const interval = window.setInterval(updateGreeting, 60_000);
    return () => window.clearInterval(interval);
  }, []);

  // TODO(Etapa 4): usar histórico consolidado para a variação por período.
  const change = mockPortfolio.changes[period];
  // TODO(Etapa 2): substituir o câmbio demonstrativo por cotação da API.
  const totalUsd = Number(summary.currentValue) / mockPortfolio.usdToBrl;
  const total = new Intl.NumberFormat("pt-BR", { style: "currency", currency }).format(totalUsd * (currency === "BRL" ? mockPortfolio.usdToBrl : 1));

  return (
    <div className="space-y-6">
    <p className="text-body-sm text-primary">Demonstração · dados fictícios · valores consolidados em BRL</p>
    <ContributionsKpis summary={summary} />
    <section aria-label="Resumo do patrimônio" className="grid min-w-0 gap-6 lg:grid-cols-[minmax(0,2fr)_minmax(280px,1fr)]">
      <div className="flex min-w-0 flex-col gap-6">
        <header>
          <h1 className="text-headline-lg font-semibold tracking-tight">{greeting}, {userName}</h1>
          <p className="mt-2 text-body-base text-on-surface-variant">Acompanhe seu patrimônio em um só lugar</p>
        </header>
        <div className="flex flex-1 flex-col rounded-3xl border border-outline-variant/30 bg-surface-container p-5 sm:p-7">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <h2 className="text-body-base font-medium text-on-surface-variant">Patrimônio total</h2>
            <select aria-label="Moeda do patrimônio" value={currency} onChange={(event) => setCurrency(event.target.value as "USD" | "BRL")} className={`rounded-full border border-outline-variant/50 bg-surface-container-high px-3 py-1.5 text-body-sm ${FOCUS}`}>
              <option value="USD">USD</option><option value="BRL">BRL</option>
            </select>
          </div>
          <p className="mt-6 break-words text-[clamp(1.8rem,4vw,3.5rem)] leading-tight font-semibold tracking-tight tabular-nums">{total}</p>
          <div className="mt-3 flex flex-wrap items-center gap-2">
            <span className={`inline-flex items-center rounded-full py-1 pl-1 pr-2.5 text-body-sm font-semibold tabular-nums ${change >= 0 ? "bg-secondary/10 text-secondary" : "bg-[#e14a65]/10 text-[#e14a65]"}`}>
              <span aria-hidden="true" className="material-symbols-outlined">{change >= 0 ? "arrow_drop_up" : "arrow_drop_down"}</span>
              {formatPercent(change)}
            </span>
            <span className="text-body-sm text-on-surface-variant">{period === "All" ? "desde o início" : `no período de ${period}`}</span>
          </div>
          <div role="group" aria-label="Período do patrimônio" className="mt-6 flex w-fit max-w-full flex-wrap gap-1 rounded-full bg-surface p-1">
            {PERIODS.map((item) => (
              <button key={item} type="button" aria-pressed={period === item} onClick={() => { setPeriod(item); onPeriodChange?.(item); }} className={`rounded-full px-3 py-2 text-body-sm font-medium transition-colors sm:px-4 ${FOCUS} ${period === item ? "bg-primary/15 text-primary" : "text-on-surface-variant hover:bg-surface-container-high"}`}>{item}</button>
            ))}
          </div>
          <div className="mt-7 grid grid-cols-2 gap-3 border-t border-outline-variant/30 pt-6">
            <Link href="/dashboard/accounts" className={`flex items-center justify-center gap-2 rounded-full bg-primary px-3 py-3 text-body-sm font-semibold text-on-primary hover:bg-primary/90 ${FOCUS}`}>
              <span aria-hidden="true" className="material-symbols-outlined">add</span>Adicionar conta
            </Link>
            <button type="button" onClick={() => setRefreshMessage("Demonstração: os saldos exibidos são simulados.")} className={`flex items-center justify-center gap-2 rounded-full border border-outline-variant px-3 py-3 text-body-sm font-medium hover:bg-surface-container-high ${FOCUS}`}>
              <span aria-hidden="true" className="material-symbols-outlined">refresh</span>Atualizar saldos
            </button>
          </div>
          <p role="status" className="mt-3 text-body-sm text-on-surface-variant">{refreshMessage}</p>
        </div>
      </div>
      <WalletsSummary accounts={accounts} />
    </section></div>
  );
}
