"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { mockUserProfile } from "@/mocks/settings";
import { mockPortfolio } from "@/mocks/portfolio";
import { summarizeAccounts } from "@/mocks/accounts";
import { useAccounts } from "./AccountsProvider";
import { percent } from "./AccountPresentation";
import { WalletsSummary } from "./WalletsSummary";
import { PortfolioAllocation } from "./PortfolioAllocation";
import styles from "./Overview.module.css";

export function PortfolioHeader({ userName = mockUserProfile.displayName }: { userName?: string }) {
  const [greeting, setGreeting] = useState("Olá");
  const [currency, setCurrency] = useState<"BRL" | "USD">("BRL");
  const { accounts, contributions } = useAccounts();
  const summary = summarizeAccounts(accounts, contributions);
  const result = Number(summary.result);
  const formatValue = (value: string) => new Intl.NumberFormat("pt-BR", {
    style: "currency", currency,
  }).format(Number(value) / (currency === "USD" ? mockPortfolio.usdToBrl : 1));

  useEffect(() => {
    function updateGreeting() {
      const hour = new Date().getHours();
      setGreeting(hour < 12 ? "Bom dia" : hour < 18 ? "Boa tarde" : "Boa noite");
    }
    updateGreeting();
    const interval = window.setInterval(updateGreeting, 60_000);
    return () => window.clearInterval(interval);
  }, []);

  return (
    <div className={styles.overview}>
      <header className={styles.heading}>
        <div>
          <p className={styles.eyebrow}>Visão geral</p>
          <h1>{greeting}, {userName}</h1>
          <p className={styles.muted}>Seu patrimônio, com cada conta no lugar.</p>
        </div>
        <Link href="/dashboard/accounts" className={styles.primaryAction}>
          <span aria-hidden="true" className="material-symbols-outlined">add</span>
          Adicionar conta
        </Link>
      </header>

      <p className={styles.demoNote}><span className={styles.demoBadge}>Demonstração</span> Dados fictícios para explorar sua visão patrimonial.</p>

      <div className={styles.grid}>
        <div className={styles.mainColumn}>
          <section aria-labelledby="portfolio-total-title" className={styles.card}>
            <div className={styles.cardHeading}>
              <h2 id="portfolio-total-title">Patrimônio total</h2>
              <label className={styles.currency}>Moeda
                <select value={currency} onChange={(event) => setCurrency(event.target.value as "BRL" | "USD")}>
                  <option value="BRL">BRL</option><option value="USD">USD</option>
                </select>
              </label>
            </div>
            <p className={styles.total}>{formatValue(summary.currentValue)}</p>
            <p className={styles.muted}>{accounts.length === 1 ? "1 conta no patrimônio" : `${accounts.length} contas no patrimônio`} · valores em {currency}</p>

            <dl className={styles.metrics}>
              <div><dt>Aportes líquidos</dt><dd>{formatValue(summary.netContributed)}</dd></div>
              <div><dt>Resultado acumulado</dt><dd className={result > 0 ? styles.positive : result < 0 ? styles.negative : undefined}>{result > 0 ? "+" : ""}{formatValue(summary.result)}</dd></div>
              <div><dt>Retorno sobre aportes</dt><dd className={result > 0 ? styles.positive : result < 0 ? styles.negative : undefined}>{result > 0 && summary.resultPct !== null ? "+" : ""}{percent(summary.resultPct)}</dd></div>
            </dl>
            <p className={styles.explanation}>Resultado = patrimônio atual − aportes líquidos. O retorno não considera o tempo de cada aporte.</p>
            {currency === "USD" && <p role="status" className={styles.exchangeNote}>Conversão demonstrativa: US$ 1 = R$ 5,20. Todos os valores desta visão usam esse câmbio.</p>}
          </section>
          <PortfolioAllocation accounts={accounts} formatValue={formatValue} />
        </div>
        <WalletsSummary accounts={accounts} formatValue={formatValue} />
      </div>
    </div>
  );
}
