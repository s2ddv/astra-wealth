import {
  calculateContributionsSummary,
  sumDecimalValues,
  type ContributionDTO,
  type FinancialAccountDTO,
  type HoldingDTO,
  type AccountKind,
} from "@astra-wealth/shared/accounts";

// TODO(Etapa 2): substituir por GET /v1/accounts e GET /v1/accounts/:id/contributions.
// Dados fictícios de demonstração; nenhuma transferência on-chain é inferida como aporte.
const date = "2026-09-01T12:00:00.000Z";
const definitions: {
  id: string;
  kind: AccountKind;
  name: string;
  institutionName: string | null;
  value: string;
}[] = [
  {
    id: "wallet-eth",
    kind: "WALLET",
    name: "Carteira principal",
    institutionName: "Ethereum",
    value: "38500",
  },
  {
    id: "wallet-sol",
    kind: "WALLET",
    name: "Carteira Solana",
    institutionName: "Solana",
    value: "12500",
  },
  {
    id: "exchange",
    kind: "EXCHANGE",
    name: "Reserva cripto",
    institutionName: "Coinbase",
    value: "21000",
  },
  {
    id: "bank",
    kind: "BANK",
    name: "Itaú — Investimentos",
    institutionName: "Itaú",
    value: "32500",
  },
  {
    id: "brokerage",
    kind: "BROKERAGE",
    name: "Carteira de longo prazo",
    institutionName: "XP",
    value: "28000",
  },
  {
    id: "manual",
    kind: "MANUAL",
    name: "Reserva de emergência",
    institutionName: null,
    value: "8500",
  },
];
const entries: [
  string,
  string,
  string,
  "DEPOSIT" | "WITHDRAWAL",
  "BRL" | "USD",
][] = [
  ["wallet-eth", "2026-02-10", "15000", "DEPOSIT", "BRL"],
  ["wallet-eth", "2026-04-10", "10000", "DEPOSIT", "BRL"],
  ["wallet-eth", "2026-07-10", "5000", "DEPOSIT", "BRL"],
  ["wallet-sol", "2026-03-05", "6000", "DEPOSIT", "BRL"],
  ["wallet-sol", "2026-05-05", "4000", "DEPOSIT", "BRL"],
  ["wallet-sol", "2026-08-05", "1000", "WITHDRAWAL", "BRL"],
  ["exchange", "2026-01-15", "2000", "DEPOSIT", "USD"],
  ["exchange", "2026-05-15", "10000", "DEPOSIT", "BRL"],
  ["exchange", "2026-08-15", "1000", "WITHDRAWAL", "BRL"],
  ["bank", "2026-02-20", "20000", "DEPOSIT", "BRL"],
  ["bank", "2026-06-20", "10000", "DEPOSIT", "BRL"],
  ["brokerage", "2026-03-12", "20000", "DEPOSIT", "BRL"],
  ["brokerage", "2026-07-12", "10000", "DEPOSIT", "BRL"],
  ["manual", "2026-04-01", "5000", "DEPOSIT", "BRL"],
  ["manual", "2026-08-01", "3500", "DEPOSIT", "BRL"],
];
export const mockContributions: ContributionDTO[] = entries.map(
  ([accountId, occurredAt, amount, kind, currency], index) => ({
    id: `contribution-${index}`,
    accountId,
    kind,
    amount,
    currency,
    fxRateToBrl: currency === "USD" ? "5.25" : null,
    occurredAt: `${occurredAt}T12:00:00.000Z`,
    note: kind === "WITHDRAWAL" ? "Retirada parcial" : "Aporte planejado",
    origin:
      accountId === "bank" || accountId === "brokerage" ? "IMPORTED" : "MANUAL",
    externalId: null,
    createdAt: date,
  }),
);
export const mockAccounts: FinancialAccountDTO[] = definitions.map(
  ({ value, ...account }) => ({
    ...account,
    baseCurrency: "BRL",
    lastSyncedAt: account.kind === "MANUAL" ? null : date,
    syncStatus:
      account.kind === "MANUAL"
        ? "MANUAL"
        : account.kind === "BROKERAGE"
          ? "NEEDS_REAUTH"
          : "ACTIVE",
    ...calculateContributionsSummary(
      mockContributions.filter((entry) => entry.accountId === account.id),
      value,
    ),
  }),
);

// TODO(Etapa 2): substituir por GET /v1/accounts/:id/holdings.
export const mockHoldings: HoldingDTO[] = [
  {
    accountId: "bank",
    assetClass: "FIXED_INCOME",
    symbol: null,
    name: "CDB 110% CDI",
    quantity: "1",
    unitPrice: "22000",
    currentValue: "22000",
    dueDate: "2028-02-20T12:00:00.000Z",
  },
  {
    accountId: "bank",
    assetClass: "FIXED_INCOME",
    symbol: null,
    name: "Tesouro Selic",
    quantity: "1",
    unitPrice: "10500",
    currentValue: "10500",
    dueDate: "2029-03-01T12:00:00.000Z",
  },
  {
    accountId: "brokerage",
    assetClass: "STOCK",
    symbol: "PETR4",
    name: "Petrobras",
    quantity: "200",
    unitPrice: "40",
    currentValue: "8000",
    dueDate: null,
  },
  {
    accountId: "brokerage",
    assetClass: "REAL_ESTATE_FUND",
    symbol: "HGLG11",
    name: "Fundo imobiliário",
    quantity: "50",
    unitPrice: "160",
    currentValue: "8000",
    dueDate: null,
  },
  {
    accountId: "brokerage",
    assetClass: "ETF",
    symbol: "IVVB11",
    name: "ETF S&P 500",
    quantity: "30",
    unitPrice: "400",
    currentValue: "12000",
    dueDate: null,
  },
  {
    accountId: "manual",
    assetClass: "CASH",
    symbol: null,
    name: "Caixa",
    quantity: "8500",
    unitPrice: "1",
    currentValue: "8500",
    dueDate: null,
  },
].map((holding, index) => ({
  ...holding,
  assetClass: holding.assetClass as HoldingDTO["assetClass"],
  id: `holding-${index}`,
  origin: holding.accountId === "manual" ? "MANUAL" : "IMPORTED",
  externalId: null,
  createdAt: date,
  updatedAt: date,
}));

// TODO(Etapa 2): consultar WalletAsset e posições de exchange; cripto não vira Holding.
export const mockCryptoPositions = [
  {
    id: "eth",
    accountId: "wallet-eth",
    name: "Ethereum",
    symbol: "ETH",
    quantity: "2.5",
    currentValue: "38500",
  },
  {
    id: "sol",
    accountId: "wallet-sol",
    name: "Solana",
    symbol: "SOL",
    quantity: "20",
    currentValue: "12500",
  },
  {
    id: "btc",
    accountId: "exchange",
    name: "Bitcoin",
    symbol: "BTC",
    quantity: "0.04",
    currentValue: "21000",
  },
];
export function summarizeAccounts(
  accounts: FinancialAccountDTO[],
  contributions: ContributionDTO[],
) {
  return calculateContributionsSummary(
    contributions,
    sumDecimalValues(accounts.map((account) => account.currentValue)),
  );
}
