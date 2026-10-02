import type {
  AccountKind,
  ContributionsSummaryDTO,
  FinancialAccountDTO,
} from "@astra-wealth/shared/accounts";
export const ACCOUNT_ICONS: Record<AccountKind, string> = {
  WALLET: "account_balance_wallet",
  EXCHANGE: "currency_exchange",
  BANK: "account_balance",
  BROKERAGE: "show_chart",
  MANUAL: "edit_note",
};
export const ACCOUNT_LABELS: Record<AccountKind, string> = {
  WALLET: "Carteiras",
  EXCHANGE: "Exchanges",
  BANK: "Bancos",
  BROKERAGE: "Corretoras",
  MANUAL: "Manuais",
};
export const money = (value: string, currency = "BRL") =>
  new Intl.NumberFormat("pt-BR", { style: "currency", currency }).format(
    Number(value),
  );
export const percent = (value: string | null) =>
  value === null
    ? "—"
    : new Intl.NumberFormat("pt-BR", {
        style: "percent",
        maximumFractionDigits: 2,
      }).format(Number(value));
export function syncLabel(account: FinancialAccountDTO) {
  // TODO(Etapa 2): calcular tempo relativo usando lastSyncedAt retornado pela API.
  return {
    ACTIVE: "Sincronizado há 2h",
    NEEDS_REAUTH: "Reautorização necessária",
    EXPIRED: "Consentimento expirado",
    ERROR: "Erro na sincronização",
    MANUAL: "Atualização manual",
  }[account.syncStatus];
}
export function ContributionsKpis({
  summary,
}: {
  summary: ContributionsSummaryDTO;
}) {
  return (
    <div className="grid gap-4 md:grid-cols-3">
      {[
        {
          label: "Total aportado (líquido)",
          value: money(summary.netContributed),
          badge: "Em BRL",
        },
        { label: "Resultado", value: money(summary.result), badge: null },
        {
          label: "Rentabilidade",
          value: percent(summary.resultPct),
          badge: null,
        },
      ].map((item) => (
        <div
          key={item.label}
          className="min-w-0 rounded-3xl border border-outline-variant/30 bg-surface-container p-5"
        >
          <div className="flex flex-wrap items-center justify-between gap-2">
            <p className="text-body-sm text-on-surface-variant">{item.label}</p>
            {item.badge ? (
              <span className="rounded-full bg-primary/10 px-2 py-1 text-xs text-primary">
                {item.badge}
              </span>
            ) : (
              <span
                className={`inline-flex items-center rounded-full px-2 py-1 text-xs ${Number(summary.result) > 0 ? "bg-secondary/10 text-secondary" : Number(summary.result) < 0 ? "bg-error/10 text-error" : "bg-surface-container-high text-on-surface-variant"}`}
              >
                {summary.resultPct !== null && (
                  <span
                    aria-hidden="true"
                    className="material-symbols-outlined"
                  >
                    {Number(summary.result) < 0
                      ? "arrow_drop_down"
                      : "arrow_drop_up"}
                  </span>
                )}
                {percent(summary.resultPct)}
              </span>
            )}
          </div>
          <p className="mt-4 break-words text-2xl font-semibold tabular-nums">
            {item.value}
          </p>
        </div>
      ))}
    </div>
  );
}
