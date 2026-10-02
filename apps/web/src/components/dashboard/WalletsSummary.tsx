"use client";

import Link from "next/link";
import type { FinancialAccountDTO } from "@astra-wealth/shared/accounts";
import { ACCOUNT_ICONS, money } from "./AccountPresentation";
import styles from "./Overview.module.css";

const STATUS_LABELS = {
  ACTIVE: "Conectada · demonstração",
  NEEDS_REAUTH: "Reautorize a conexão",
  EXPIRED: "Renove o consentimento",
  ERROR: "Revise a conexão",
  MANUAL: "Atualização manual",
};
const needsAttention = (account: FinancialAccountDTO) => ["NEEDS_REAUTH", "EXPIRED", "ERROR"].includes(account.syncStatus);

export function WalletsSummary({ accounts, formatValue = money }: {
  accounts: FinancialAccountDTO[];
  formatValue?: (value: string) => string;
}) {
  const attentionCount = accounts.filter(needsAttention).length;
  const sortedAccounts = [...accounts].sort((a, b) => Number(needsAttention(b)) - Number(needsAttention(a)));

  return (
    <section aria-labelledby="hero-accounts-title" className={styles.card}>
      <header className={styles.cardHeading}>
        <h2 id="hero-accounts-title">Suas contas</h2>
        <span className={styles.count}>{accounts.length}</span>
      </header>
      {attentionCount > 0 && <p className={styles.attention}>{attentionCount === 1 ? "1 conta precisa de atenção" : `${attentionCount} contas precisam de atenção`}</p>}
      {accounts.length === 0 ? <div className={styles.empty}><p>Nenhuma conta adicionada.</p><p>Adicione uma conta para começar a acompanhar seu patrimônio.</p></div> : <ul className={styles.accounts}>
        {sortedAccounts.map((account) => <li key={account.id}>
          <Link href={`/dashboard/accounts/${account.id}`} className={styles.accountLink}>
            <span aria-hidden="true" className={`material-symbols-outlined ${styles.accountIcon}`}>{ACCOUNT_ICONS[account.kind]}</span>
            <div className={styles.accountContent}>
              <p className={styles.accountName}>{account.name}</p>
              <p className={styles.accountValue}>{formatValue(account.currentValue)}</p>
              <p className={needsAttention(account) ? styles.accountWarning : styles.accountStatus}>{STATUS_LABELS[account.syncStatus]}</p>
            </div>
            <span aria-hidden="true" className={styles.chevron}>›</span>
          </Link>
        </li>)}
      </ul>}
      <Link href="/dashboard/accounts" className={styles.textAction}>{accounts.length === 0 ? "Adicionar primeira conta" : "Gerenciar contas"}<span aria-hidden="true">→</span></Link>
    </section>
  );
}
