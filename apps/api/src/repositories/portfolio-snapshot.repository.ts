import type { PrismaClient, SnapshotSource } from "@astra-wealth/database";

export class PortfolioSnapshotRepository {
  constructor(private readonly db: PrismaClient) {}

  create(userId: string, totalUsd: number | string, source: SnapshotSource = "WALLET_REFRESH") {
    return this.db.portfolioSnapshot.create({
      data: { userId, totalUsd, source },
    });
  }

  findByUser(userId: string, limit = 90) {
    return this.db.portfolioSnapshot.findMany({
      where: { userId },
      orderBy: { createdAt: "desc" },
      take: limit,
    });
  }

  findByUserId(userId: string, limit = 90) {
    return this.findByUser(userId, limit);
  }

  findLatestByUser(userId: string) {
    return this.db.portfolioSnapshot.findFirst({
      where: { userId },
      orderBy: { createdAt: "desc" },
    });
  }
}