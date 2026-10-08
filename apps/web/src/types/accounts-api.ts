import type { AccountKind, FiatCurrency, HoldingDTO, ContributionDTO } from "@astra-wealth/shared/accounts";
export interface AccountRecord {
  id: string;
  name: string;
  institutionName: string | null;
  kind: AccountKind;
  baseCurrency: FiatCurrency;
  walletId: string | null;
}
export interface AccountDetail extends AccountRecord {
  holdings: HoldingDTO[];
  contributions: ContributionDTO[];
}
