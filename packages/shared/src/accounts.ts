import { z } from "zod";

export type AccountKind =
  | "WALLET"
  | "EXCHANGE"
  | "BANK"
  | "BROKERAGE"
  | "MANUAL";
export type AssetClass =
  | "CRYPTO"
  | "STOCK"
  | "REAL_ESTATE_FUND"
  | "ETF"
  | "FIXED_INCOME"
  | "MUTUAL_FUND"
  | "PENSION"
  | "CASH"
  | "OTHER";
export type ContributionKind = "DEPOSIT" | "WITHDRAWAL";
export type FiatCurrency = "BRL" | "USD";
export type DataOrigin = "MANUAL" | "IMPORTED";
export type ConnectionStatus = "ACTIVE" | "NEEDS_REAUTH" | "EXPIRED" | "ERROR";
/** Valores monetários do resumo sempre em BRL, serializados como string. */
export interface ContributionsSummaryDTO {
  totalContributed: string;
  totalWithdrawn: string;
  netContributed: string;
  currentValue: string;
  result: string;
  /** Fração, não percentual: "0.1" representa 10%. */
  resultPct: string | null;
}
export interface FinancialAccountDTO extends ContributionsSummaryDTO {
  id: string;
  kind: AccountKind;
  name: string;
  institutionName: string | null;
  baseCurrency: FiatCurrency;
  lastSyncedAt?: string | null;
  syncStatus: ConnectionStatus | "MANUAL";
}
export interface HoldingDTO {
  id: string;
  accountId: string;
  assetClass: AssetClass;
  symbol: string | null;
  name: string;
  quantity: string;
  unitPrice: string | null;
  currentValue: string;
  dueDate: string | null;
  origin: DataOrigin;
  externalId: string | null;
  createdAt: string;
  updatedAt: string;
}
export interface ContributionDTO {
  id: string;
  accountId: string;
  kind: ContributionKind;
  amount: string;
  currency: FiatCurrency;
  fxRateToBrl: string | null;
  occurredAt: string;
  note: string | null;
  origin: DataOrigin;
  externalId: string | null;
  createdAt: string;
}

const positiveAmount = z
  .string()
  .regex(/^\d{1,18}(?:\.\d{1,10})?$/, "Informe um valor decimal válido.")
  .refine((value) => /[1-9]/.test(value), "O valor deve ser maior que zero.");
const positiveRate = z
  .string()
  .regex(/^\d{1,10}(?:\.\d{1,8})?$/, "Informe uma cotação válida.")
  .refine((value) => /[1-9]/.test(value), "A cotação deve ser positiva.");
export const createContributionSchema = z
  .object({
    accountId: z.string().trim().min(1, "Selecione uma conta."),
    kind: z.enum(["DEPOSIT", "WITHDRAWAL"]),
    amount: positiveAmount,
    currency: z.enum(["BRL", "USD"]),
    fxRateToBrl: positiveRate.nullish(),
    occurredAt: z
      .string()
      .datetime({ offset: true })
      .refine(
        (value) => Date.parse(value) <= Date.now(),
        "A data não pode ser futura.",
      ),
    note: z.string().trim().max(1000).optional(),
  })
  .superRefine((value, context) => {
    if (value.currency !== "BRL" && !value.fxRateToBrl)
      context.addIssue({
        code: "custom",
        path: ["fxRateToBrl"],
        message: "Informe a cotação para BRL na data do aporte.",
      });
  });
export type CreateContributionInput = z.infer<typeof createContributionSchema>;

// Escala fixa evita arredondamentos binários; arredondamento monetário ocorre só na apresentação.
const SCALE = 10n ** 10n;
function decimal(value: string): bigint {
  if (!/^-?\d+(?:\.\d{1,10})?$/.test(value))
    throw new Error("Valor decimal inválido.");
  const negative = value.startsWith("-");
  const [whole = "0", fraction = ""] = value.replace(/^-/, "").split(".");
  return (
    (BigInt(whole) * SCALE + BigInt(fraction.padEnd(10, "0"))) *
    (negative ? -1n : 1n)
  );
}
function serialize(value: bigint): string {
  const absolute = value < 0n ? -value : value;
  const fraction = (absolute % SCALE)
    .toString()
    .padStart(10, "0")
    .replace(/0+$/, "");
  return `${value < 0n ? "-" : ""}${absolute / SCALE}${fraction ? `.${fraction}` : ""}`;
}
export function sumDecimalValues(values: readonly string[]): string {
  return serialize(values.reduce((sum, value) => sum + decimal(value), 0n));
}

/** Valores consolidados em BRL. Aportes on-chain são manuais, nunca inferidos de transferências.
 * Líquido = depósitos − retiradas; resultado = valor atual − líquido.
 * Rentabilidade = resultado / líquido; sem base positiva, retorna null.
 * USD exige cotação histórica positiva; conversão é truncada em dez casas decimais.
 */
export function calculateContributionsSummary(
  contributions: readonly Pick<
    ContributionDTO,
    "kind" | "amount" | "currency" | "fxRateToBrl"
  >[],
  currentValue: string,
): ContributionsSummaryDTO {
  let deposits = 0n;
  let withdrawals = 0n;
  for (const contribution of contributions) {
    let amount = decimal(contribution.amount);
    if (amount <= 0n) throw new Error("O valor deve ser positivo.");
    if (contribution.currency !== "BRL") {
      if (!contribution.fxRateToBrl)
        throw new Error("Cotação histórica para BRL obrigatória.");
      const rate = decimal(contribution.fxRateToBrl);
      if (rate <= 0n) throw new Error("A cotação deve ser positiva.");
      amount = (amount * rate) / SCALE;
    }
    if (contribution.kind === "DEPOSIT") deposits += amount;
    else withdrawals += amount;
  }
  const net = deposits - withdrawals;
  const current = decimal(currentValue);
  const result = current - net;
  return {
    totalContributed: serialize(deposits),
    totalWithdrawn: serialize(withdrawals),
    netContributed: serialize(net),
    currentValue: serialize(current),
    result: serialize(result),
    resultPct: net > 0n ? serialize((result * SCALE) / net) : null,
  };
}
