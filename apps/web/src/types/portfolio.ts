import type { ChainId } from "./dashboard";

export type PortfolioPeriod = "24h" | "7d" | "30d" | "1A" | "All";
export type PortfolioChain = ChainId | "bitcoin";

export interface SummaryWallet {
  id: string;
  nickname?: string;
  address: string;
  chain: PortfolioChain;
  valueUsd: number;
  status: "synced" | "error";
}
