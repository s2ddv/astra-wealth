import type { FastifyInstance } from "fastify";
import { UserRepository } from "../../repositories/user.repository.js";
import { requireUserId } from "../../lib/dev-auth.js";
import { MeWalletsService } from "./me-wallets.service.js";
import { MeWatchlistsService } from "./me-watchlists.service.js";
import { MePortfolioService } from "./me-portfolio.service.js";
import {
  addWatchlistItemSchema,
  createSnapshotSchema,
  createWalletSchema,
  createWatchlistSchema,
  snapshotQuerySchema,
  walletIdParamsSchema,
  watchlistIdParamsSchema,
  watchlistItemParamsSchema,
} from "./me.schema.js";

export default async function meRoutes(fastify: FastifyInstance) {
  const userRepo = new UserRepository(fastify.prisma);
  const walletsService = new MeWalletsService(fastify);
  const watchlistsService = new MeWatchlistsService(fastify);
  const portfolioService = new MePortfolioService(fastify);

  async function auth(request: Parameters<typeof requireUserId>[0], reply: Parameters<typeof requireUserId>[1]) {
    return requireUserId(request, reply, userRepo);
  }

  // Wallets
  fastify.get("/v1/me/wallets", async (request, reply) => {
    const userId = await auth(request, reply);
    if (!userId) return;
    return reply.send(await walletsService.list(userId));
  });

  fastify.post("/v1/me/wallets", async (request, reply) => {
    const userId = await auth(request, reply);
    if (!userId) return;

    const parsed = createWalletSchema.safeParse(request.body);
    if (!parsed.success) {
      return reply.status(400).send({ error: parsed.error.flatten() });
    }

    try {
      const wallet = await walletsService.create(userId, parsed.data);
      return reply.status(201).send(wallet);
    } catch (err: unknown) {
      if (
        err &&
        typeof err === "object" &&
        "code" in err &&
        err.code === "P2002"
      ) {
        return reply.status(409).send({ error: "Wallet already exists" });
      }
      throw err;
    }
  });

  fastify.delete<{ Params: { id: string } }>(
    "/v1/me/wallets/:id",
    async (request, reply) => {
      const userId = await auth(request, reply);
      if (!userId) return;

      const params = walletIdParamsSchema.safeParse(request.params);
      if (!params.success) {
        return reply.status(400).send({ error: "Invalid wallet id" });
      }

      let removed: boolean;
      try {
        removed = await walletsService.remove(params.data.id, userId);
      } catch (error) {
        if (error && typeof error === "object" && "code" in error && error.code === "WALLET_LINKED_TO_ACCOUNT") {
          return reply.status(409).send({ code: error.code, error: "Remove the linked financial account before deleting this wallet" });
        }
        throw error;
      }
      if (!removed) {
        return reply.status(404).send({ error: "Wallet not found" });
      }
      return reply.status(204).send();
    }
  );

  fastify.post<{ Params: { id: string } }>(
    "/v1/me/wallets/:id/sync",
    async (request, reply) => {
      const userId = await auth(request, reply);
      if (!userId) return;

      const params = walletIdParamsSchema.safeParse(request.params);
      if (!params.success) {
        return reply.status(400).send({ error: "Invalid wallet id" });
      }

      const wallet = await walletsService.sync(params.data.id, userId);
      if (!wallet) {
        return reply.status(404).send({ error: "Wallet not found" });
      }
      return reply.send(wallet);
    }
  );

  // Watchlists
  fastify.get("/v1/me/watchlists", async (request, reply) => {
    const userId = await auth(request, reply);
    if (!userId) return;
    return reply.send(await watchlistsService.list(userId));
  });

  fastify.post("/v1/me/watchlists", async (request, reply) => {
    const userId = await auth(request, reply);
    if (!userId) return;

    const parsed = createWatchlistSchema.safeParse(request.body);
    if (!parsed.success) {
      return reply.status(400).send({ error: parsed.error.flatten() });
    }

    try {
      const watchlist = await watchlistsService.create(userId, parsed.data.name);
      return reply.status(201).send(watchlist);
    } catch (err: unknown) {
      if (
        err &&
        typeof err === "object" &&
        "code" in err &&
        err.code === "P2002"
      ) {
        return reply.status(409).send({ error: "Watchlist name already exists" });
      }
      throw err;
    }
  });

  fastify.delete<{ Params: { id: string } }>(
    "/v1/me/watchlists/:id",
    async (request, reply) => {
      const userId = await auth(request, reply);
      if (!userId) return;

      const params = watchlistIdParamsSchema.safeParse(request.params);
      if (!params.success) {
        return reply.status(400).send({ error: "Invalid watchlist id" });
      }

      const removed = await watchlistsService.remove(params.data.id, userId);
      if (!removed) {
        return reply.status(404).send({ error: "Watchlist not found" });
      }
      return reply.status(204).send();
    }
  );

  fastify.post<{ Params: { id: string } }>(
    "/v1/me/watchlists/:id/items",
    async (request, reply) => {
      const userId = await auth(request, reply);
      if (!userId) return;

      const params = watchlistIdParamsSchema.safeParse(request.params);
      if (!params.success) {
        return reply.status(400).send({ error: "Invalid watchlist id" });
      }

      const parsed = addWatchlistItemSchema.safeParse(request.body);
      if (!parsed.success) {
        return reply.status(400).send({ error: parsed.error.flatten() });
      }

      try {
        const item = await watchlistsService.addItem(
          params.data.id,
          userId,
          parsed.data.coinId
        );
        if (!item) {
          return reply.status(404).send({ error: "Watchlist not found" });
        }
        return reply.status(201).send(item);
      } catch (err: unknown) {
        if (
          err &&
          typeof err === "object" &&
          "code" in err &&
          err.code === "P2002"
        ) {
          return reply.status(409).send({ error: "Coin already in watchlist" });
        }
        throw err;
      }
    }
  );

  fastify.delete<{ Params: { id: string; coinId: string } }>(
    "/v1/me/watchlists/:id/items/:coinId",
    async (request, reply) => {
      const userId = await auth(request, reply);
      if (!userId) return;

      const params = watchlistItemParamsSchema.safeParse(request.params);
      if (!params.success) {
        return reply.status(400).send({ error: "Invalid params" });
      }

      const removed = await watchlistsService.removeItem(
        params.data.id,
        userId,
        params.data.coinId
      );
      if (!removed) {
        return reply.status(404).send({ error: "Watchlist or item not found" });
      }
      return reply.status(204).send();
    }
  );

  // Portfolio snapshots
  fastify.get<{ Querystring: { limit?: string } }>(
    "/v1/me/portfolio/snapshots",
    async (request, reply) => {
      const userId = await auth(request, reply);
      if (!userId) return;

      const query = snapshotQuerySchema.safeParse(request.query);
      if (!query.success) {
        return reply.status(400).send({ error: query.error.flatten() });
      }

      return reply.send(
        await portfolioService.listSnapshots(userId, query.data.limit)
      );
    }
  );

  fastify.post("/v1/me/portfolio/snapshots", async (request, reply) => {
    const userId = await auth(request, reply);
    if (!userId) return;

    const parsed = createSnapshotSchema.safeParse(request.body);
    if (!parsed.success) {
      return reply.status(400).send({ error: parsed.error.flatten() });
    }

    const snapshot = await portfolioService.createSnapshot(
      userId,
      parsed.data.totalUsd
    );
    return reply.status(201).send(snapshot);
  });
}