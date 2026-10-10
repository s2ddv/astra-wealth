export {
  PrismaClient,
  Chain,
  Exchange,
  type User,
  type Wallet,
  type WalletAsset,
  type Watchlist,
  type WatchlistItem,
  type PortfolioSnapshot,
  type ExchangeConnection,
  type SnapshotSource,
} from "./generated/client/index.js";

export * as Prisma from "./generated/client/index.js";

import { PrismaClient } from "./generated/client/index.js";

const globalForPrisma = globalThis as unknown as { prisma?: PrismaClient };

export const prisma =
  globalForPrisma.prisma ??
  new PrismaClient({
    log:
      process.env.NODE_ENV === "development"
        ? ["query", "error", "warn"]
        : ["error"],
  });

if (process.env.NODE_ENV !== "production") {
  globalForPrisma.prisma = prisma;
}
