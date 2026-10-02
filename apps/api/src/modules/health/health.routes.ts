import type { FastifyPluginAsync } from "fastify";

export const healthRoutes: FastifyPluginAsync = async (app) => {
  app.get(
    "/",
    {
      schema: {
        response: {
          200: {
            type: "object",
            required: ["status", "service", "timestamp", "checks"],
            properties: {
              status: { type: "string" },
              service: { type: "string" },
              timestamp: { type: "string" },
              checks: {
                type: "object",
                properties: {
                  database: { type: "string" },
                  redis: { type: "string" },
                },
              },
            },
          },
        },
      },
    },
    async () => {
      let database: "ok" | "error" = "ok";
      let redis: "ok" | "error" = "ok";

      try {
        await app.prisma.$queryRaw`SELECT 1`;
      } catch {
        database = "error";
      }

      try {
        const pong = await app.redis.ping();
        if (pong !== "PONG") redis = "error";
      } catch {
        redis = "error";
      }

      const healthy = database === "ok" && redis === "ok";

      return {
        status: healthy ? "ok" : "degraded",
        service: "astra-wealth-api",
        timestamp: new Date().toISOString(),
        checks: { database, redis },
      };
    }
  );
};
