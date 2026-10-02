import type { PrismaClient, Exchange } from "@astra-wealth/database";

export class ExchangeConnectionRepository {
  constructor(private readonly db: PrismaClient) {}

  findAllByUserId(userId: string) {
    return this.db.exchangeConnection.findMany({
      where: { userId },
      orderBy: { createdAt: "desc" },
    });
  }

  findById(id: string, userId: string) {
    return this.db.exchangeConnection.findFirst({
      where: { id, userId },
    });
  }

  findByExchange(userId: string, exchange: Exchange) {
    return this.db.exchangeConnection.findUnique({
      where: {
        userId_exchange: { userId, exchange },
      },
    });
  }

  create(
    userId: string,
    exchange: Exchange,
    encryptedKey: string,
    encryptedSecret: string,
  ) {
    return this.db.exchangeConnection.create({
      data: { userId, exchange, encryptedKey, encryptedSecret },
    });
  }

  updateCredentials(
    id: string,
    userId: string,
    encryptedKey: string,
    encryptedSecret: string,
  ) {
    return this.db.exchangeConnection.updateMany({
      where: { id, userId },
      data: { encryptedKey, encryptedSecret },
    });
  }

  delete(id: string, userId: string) {
    return this.db.exchangeConnection.deleteMany({
      where: { id, userId },
    });
  }
}