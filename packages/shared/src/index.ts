export type AssetSymbol = "BTC" | "ETH" | "USDC" | "USDT" | string;
export * from "./market.js";
export * from "./wallet.js";
export * from "./database.js";

export interface ApiResponse<T> {
  data: T;
  meta?: {
    cachedAt: string;
    source: string;
  };
}

export interface MarketAsset {
  id: string;
  symbol: AssetSymbol;
  name: string;
  priceUsd: number;
  change24h: number;
  marketCapUsd?: number;
  updatedAt: string;
}

export interface Transfer {
  id: string;
  chain: "ethereum" | "polygon" | "base" | "arbitrum";
  asset: AssetSymbol;
  amount: number;
  valueUsd?: number;
  from: string;
  to: string;
  transactionHash: string;
  occurredAt: string;
}

export interface NewsItem {
  id: string;
  title: string;
  url: string;
  source: string;
  category: "crypto" | "world";
  publishedAt: string;
}

export type { NewsArticleDto, NewsPageDto } from "./generated/rust/NewsDto.js";

export * from "./accounts.js";
