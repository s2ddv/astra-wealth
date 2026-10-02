import { randomUUID } from "node:crypto";
import type { PrismaClient } from "@astra-wealth/database";

export class UserRepository {
  constructor(private readonly db: PrismaClient) {}

  create(data: { id?: string; email: string; name?: string | null; authId: string }) {
    return this.db.user.create({
      data: {
        id: data.id ?? randomUUID(),
        email: data.email,
        name: data.name ?? null,
        authId: data.authId,
      },
    });
  }

  findByEmail(email: string) {
    return this.db.user.findUnique({
      where: { email },
    });
  }

  findById(id: string) {
    return this.db.user.findUnique({
      where: { id },
    });
  }

  findByIdWithRelations(id: string) {
    return this.db.user.findUnique({
      where: { id },
      include: {
        wallets: true,
        watchlists: true,
        exchangeConnections: true,
      },
    });
  }
}