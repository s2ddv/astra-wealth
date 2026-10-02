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
      data,
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
    return this.db.wallet.deleteMany({
      where: { id, userId },
    });
  }
}