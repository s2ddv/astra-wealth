import type { Chain, PrismaClient } from "@astra-wealth/database";

export class WalletRepository {
  constructor(private readonly db: PrismaClient) {}

  create(data: {
    userId: string;
    address: string;
    chain: Chain;
    nickname?: string | null;
  }) {
    return this.db.wallet.create({
      data: {
        ...data,
        address: ["ETHEREUM", "POLYGON", "ARBITRUM", "BASE"].includes(data.chain)
          ? data.address.toLowerCase()
          : data.address,
      },
      include: { assets: true },
    });
  }

  findByUserId(userId: string) {
    return this.db.wallet.findMany({
      where: { userId },
      include: { assets: true },
      orderBy: { createdAt: "desc" },
    });
  }

  findById(id: string, userId: string) {
    return this.db.wallet.findFirst({
      where: { id, userId },
      include: { assets: true },
    });
  }

  findByIdForUser(id: string, userId: string) {
    return this.db.wallet.findFirst({
      where: { id, userId },
      include: { assets: true },
    });
  }

  updateNickname(id: string, userId: string, nickname: string) {
    return this.db.wallet.updateMany({
      where: { id, userId },
      data: { nickname },
    });
  }

  delete(id: string, userId: string) {
    return this.db.$transaction(async (tx) => {
      const owned = await tx.$queryRaw<{ id: string }[]>`
        SELECT id FROM wallets WHERE id = ${id} AND "userId" = ${userId} FOR UPDATE`;
      if (!owned.length) return { count: 0 };
      const linked = await tx.financialAccount.findUnique({ where: { walletId: id } });
      if (linked) {
        throw Object.assign(new Error("Wallet is linked to a financial account"), {
          code: "WALLET_LINKED_TO_ACCOUNT",
        });
      }
      return tx.wallet.deleteMany({ where: { id, userId } });
    });
  }
}