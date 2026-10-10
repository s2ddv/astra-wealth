export interface FxRate {
  pair: "USD/BRL";
  source: string;
  requestedDate: string;
  quote: { rate: string; effectiveDate: string; publishedAt: string } | null;
  status: "available" | "previous_close" | "stale" | "missing" | "unavailable";
  fetchedAt: string;
}
export interface PortfolioSnapshot {
  version: number;
  observedAt: string;
  valuedAt: string;
  knownValueBrl: string | null;
  complete: boolean;
  netContributionsBrl: string | null;
  contributionsComplete: boolean;
  resultBrl: string | null;
  positions: Array<{
    id: string; accountId: string | null; walletId: string | null; name: string;
    valueBrl: string | null; source: string; balanceUpdatedAt: string; issue: string | null;
    inputValue: string | null; currency: string | null; unitPriceBrl: string | null;
    priceFetchedAt: string | null; fx: FxRate | null;
  }>;
  contributions: Array<{
    id: string; valueBrl: string | null; rate: string | null; source: string;
    occurredOn: string; fx: FxRate | null;
  }>;
}
