import type { PrismaClient, Prisma } from "@astra-wealth/database";

export class WalletAssetRepository {
  constructor(private readonly db: PrismaClient) {}

  findByWallet(walletId: string) {
    return this.db.walletAsset.findMany({
      where: { walletId },
    });
  }

  upsert(
    walletId: string,
    symbol: string,
    amount: number | string,
    options?: {
      contractAddress?: string | null;
      isNative?: boolean;
      usdValue?: number | string | null;
      name?: string | null;
    },
  ) {
    return this.db.walletAsset.upsert({
      where: {
        walletId_symbol: { walletId, symbol },
      },
      create: {
        walletId,
        symbol,
        amount,
        contractAddress: options?.contractAddress ?? null,
        isNative: options?.isNative ?? false,
        usdValue: options?.usdValue ?? null,
        name: options?.name ?? null,
      },
      update: {
        amount,
        usdValue: options?.usdValue ?? null,
        name: options?.name ?? null,
      },
    });
  }

  deleteByWallet(walletId: string) {
    return this.db.walletAsset.deleteMany({
      where: { walletId },
    });
  }
}