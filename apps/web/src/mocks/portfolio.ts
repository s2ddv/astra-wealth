import { mockWallets } from "./wallets";
import type { PortfolioPeriod, SummaryWallet } from "@/types/portfolio";

// MOCK — substituir na Fase 6 (Wallet Tracking) e Fase 7 (Portfolio Engine)
export const mockPortfolioWallets: SummaryWallet[] = mockWallets.map((wallet) => ({
  ...wallet,
  status: "synced",
}));

// MOCK — substituir na Fase 6 (Wallet Tracking) e Fase 7 (Portfolio Engine)
export const mockPortfolio = {
  totalUsd: mockPortfolioWallets.reduce((total, wallet) => total + wallet.valueUsd, 0),
  usdToBrl: 5.2,
  changes: { "24h": 1.84, "7d": -2.36, "30d": 8.72, "1A": 24.18, All: 32.65 } satisfies Record<PortfolioPeriod, number>,
};
