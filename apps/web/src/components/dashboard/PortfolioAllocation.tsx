import type { AccountKind, FinancialAccountDTO } from "@astra-wealth/shared/accounts";
import { sumDecimalValues } from "@astra-wealth/shared/accounts";
import { ACCOUNT_LABELS } from "./AccountPresentation";
import styles from "./Overview.module.css";

const KINDS: AccountKind[] = ["WALLET", "EXCHANGE", "BANK", "BROKERAGE", "MANUAL"];

export function PortfolioAllocation({ accounts, formatValue }: {
  accounts: FinancialAccountDTO[];
  formatValue: (value: string) => string;
}) {
  const groups = KINDS.map((kind) => ({
    kind,
    value: sumDecimalValues(accounts.filter((account) => account.kind === kind).map((account) => account.currentValue)),
  })).filter((group) => Number(group.value) !== 0);
  const total = Number(sumDecimalValues(groups.map((group) => group.value)));
  const canShowShares = total > 0 && groups.every((group) => Number(group.value) >= 0);

  return (
    <section className={styles.card} aria-labelledby="allocation-title">
      <div className={styles.cardHeading}><h2 id="allocation-title">Onde está seu patrimônio</h2><span className={styles.muted}>Por tipo de conta</span></div>
      {groups.length === 0 ? <p className={styles.empty}>A distribuição aparece quando suas contas têm saldo.</p> : <>
        {canShowShares && <div className={styles.allocationBar} aria-hidden="true">
          {groups.map((group) => <span key={group.kind} data-kind={group.kind} style={{ flexGrow: Number(group.value) / total }} />)}
        </div>}
        <ul className={styles.allocationList}>
          {groups.map((group) => <li key={group.kind}>
            <span className={styles.allocationLabel}><span className={styles.swatch} data-kind={group.kind} aria-hidden="true" />{ACCOUNT_LABELS[group.kind]}</span>
            <span className={styles.allocationValues}><strong>{formatValue(group.value)}</strong><span className={styles.muted}>{canShowShares ? new Intl.NumberFormat("pt-BR", { style: "percent", maximumFractionDigits: 1 }).format(Number(group.value) / total) : "—"}</span></span>
          </li>)}
        </ul>
      </>}
    </section>
  );
}
