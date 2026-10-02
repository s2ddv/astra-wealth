import type { PrismaClient } from "@astra-wealth/database";

export class WatchlistRepository {
  constructor(private readonly db: PrismaClient) {}

  findAllByUserId(userId: string) {
    return this.db.watchlist.findMany({
      where: { userId },
      include: { items: true },
      orderBy: { createdAt: "desc" },
    });
  }

  findById(id: string, userId: string) {
    return this.db.watchlist.findFirst({
      where: { id, userId },
      include: { items: true },
    });
  }

  create(userId: string, name: string) {
    return this.db.watchlist.create({
      data: { userId, name },
      include: { items: true },
    });
  }

  rename(id: string, userId: string, name: string) {
    return this.db.watchlist.updateMany({
      where: { id, userId },
      data: { name },
    });
  }

  delete(id: string, userId: string) {
    return this.db.watchlist.deleteMany({
      where: { id, userId },
    });
  }

  addItem(watchlistId: string, coinId: string) {
    return this.db.watchlistItem.create({
      data: { coinId, watchlistId },
    });
  }

  removeItem(watchlistId: string, coinId: string) {
    return this.db.watchlistItem.deleteMany({
      where: { watchlistId, coinId },
    });
  }
}