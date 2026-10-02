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

type InstitutionIcon = {
  label: string;
  glyph?: string;
  mark?: string;
  background: string;
  color: string;
};

const FALLBACK_ICON_BY_KIND: Record<AccountKind, InstitutionIcon> = {
  WALLET: {
    label: "Carteira",
    glyph: ACCOUNT_ICONS.WALLET,
    background: "#2d2438",
    color: "#d1bcff",
  },
  EXCHANGE: {
    label: "Exchange",
    glyph: ACCOUNT_ICONS.EXCHANGE,
    background: "#252b3a",
    color: "#a9c7e8",
  },
  BANK: {
    label: "Banco",
    glyph: ACCOUNT_ICONS.BANK,
    background: "#202f34",
    color: "#9fd8c5",
  },
  BROKERAGE: {
    label: "Corretora",
    glyph: ACCOUNT_ICONS.BROKERAGE,
    background: "#332b21",
    color: "#e6c99e",
  },
  MANUAL: {
    label: "Manual",
    glyph: ACCOUNT_ICONS.MANUAL,
    background: "#2f2d36",
    color: "#cac4d0",
  },
};

const INSTITUTION_ICONS: Record<string, InstitutionIcon> = {
  ethereum: {
    label: "Ethereum",
    mark: "Ξ",
    background: "#eef0ff",
    color: "#4f5bd5",
  },
  solana: {
    label: "Solana",
    mark: "S",
    background: "#182d2c",
    color: "#5ff7d2",
  },
  coinbase: {
    label: "Coinbase",
    mark: "C",
    background: "#1652f0",
    color: "#ffffff",
  },
  binance: {
    label: "Binance",
    mark: "B",
    background: "#f3ba2f",
    color: "#181a20",
  },
  kraken: {
    label: "Kraken",
    mark: "K",
    background: "#5841d8",
    color: "#ffffff",
  },
  itau: {
    label: "Itaú",
    mark: "it",
    background: "#ff7800",
    color: "#141a6b",
  },
  "banco do brasil": {
    label: "Banco do Brasil",
    mark: "BB",
    background: "#ffe000",
    color: "#144a9a",
  },
  nubank: {
    label: "Nubank",
    mark: "Nu",
    background: "#820ad1",
    color: "#ffffff",
  },
  inter: {
    label: "Inter",
    mark: "in",
    background: "#ff7a00",
    color: "#ffffff",
  },
  xp: {
    label: "XP",
    mark: "XP",
    background: "#101010",
    color: "#f6c64f",
  },
  rico: {
    label: "Rico",
    mark: "R",
    background: "#0f7a55",
    color: "#ffffff",
  },
  clear: {
    label: "Clear",
    mark: "C",
    background: "#222222",
    color: "#7bdcb5",
  },
  btg: {
    label: "BTG Pactual",
    mark: "BTG",
    background: "#111827",
    color: "#ffffff",
  },
};

function normalizeInstitution(value: string) {
  return value
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase();
}

export function getAccountInstitutionIcon(
  account: Pick<FinancialAccountDTO, "kind" | "institutionName">,
) {
  const institution = account.institutionName
    ? normalizeInstitution(account.institutionName)
    : "";
  const matchedKey = Object.keys(INSTITUTION_ICONS).find((key) =>
    institution.includes(key),
  );
  const matchedIcon = matchedKey ? INSTITUTION_ICONS[matchedKey] : undefined;
  return matchedIcon ?? FALLBACK_ICON_BY_KIND[account.kind];
}

export function AccountInstitutionIcon({
  account,
  className = "",
}: {
  account: Pick<FinancialAccountDTO, "kind" | "institutionName">;
  className?: string | undefined;
}) {
  const icon = getAccountInstitutionIcon(account);
  const baseClass =
    "inline-flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl text-sm font-bold leading-none";
  return (
    <span
      aria-hidden="true"
      title={icon.label}
      className={`${baseClass} ${icon.glyph ? "material-symbols-outlined text-[24px] font-normal" : ""} ${className}`}
      style={{ backgroundColor: icon.background, color: icon.color }}
    >
      {icon.glyph ?? icon.mark ?? "?"}
    </span>
  );
}

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
