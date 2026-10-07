import type { PrismaClient } from "@astra-wealth/database";

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
    const assetKey = options?.isNative
      ? "native"
      : options?.contractAddress?.toLowerCase();
    if (!assetKey) throw new Error("Token assets require a contract address");
    return this.db.$transaction(async (tx) => {
      // Match the Rust sync lock and mark legacy writes as incomplete.
      await tx.$queryRaw`SELECT id FROM wallets WHERE id = ${walletId} FOR UPDATE`;
      await tx.wallet.update({
        where: { id: walletId },
        data: { lastSyncComplete: false, lastValuationComplete: false },
      });
      return tx.walletAsset.upsert({
      where: {
        walletId_assetKey: { walletId, assetKey },
      },
      create: {
        walletId,
        assetKey,
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
    });
  }

  deleteByWallet(walletId: string) {
    return this.db.walletAsset.deleteMany({
      where: { walletId },
    });
  }
}