import fp from "fastify-plugin";
import { createClient } from "@supabase/supabase-js";
import type { FastifyInstance, FastifyRequest, FastifyReply } from "fastify";

declare module "fastify" {
  interface FastifyRequest {
    userId: string | null;
    authId: string | null;
  }
}

const PUBLIC_ROUTES = ["/v1/health", "/v1/market", "/v1/wallet"];

function isPublicRoute(url: string): boolean {
  const path = url.split("?", 1)[0]!;
  return PUBLIC_ROUTES.some((route) => path === route || path.startsWith(`${route}/`));
}

export default fp(async (app: FastifyInstance) => {
  const supabaseUrl = process.env.SUPABASE_URL?.trim();
  const supabaseServiceKey = process.env.SUPABASE_SERVICE_ROLE_KEY?.trim();
  const allowDevAuth = process.env.NODE_ENV === "development" && process.env.ALLOW_DEV_AUTH === "true";
  const devUserId = process.env.DEV_USER_ID?.trim();

  // Partial configuration is always an error. Missing configuration is allowed
  // only with an explicit development opt-in, never by default.
  if ((!supabaseUrl || !supabaseServiceKey) && (!allowDevAuth || supabaseUrl || supabaseServiceKey)) {
    throw new Error("Authentication requires SUPABASE_URL and SUPABASE_SERVICE_ROLE_KEY; local development requires NODE_ENV=development and ALLOW_DEV_AUTH=true");
  }
  const supabaseAdmin = supabaseUrl && supabaseServiceKey
    ? createClient(supabaseUrl, supabaseServiceKey) : null;
  app.decorateRequest("userId", null);
  app.decorateRequest("authId", null);

  app.addHook("onRequest", async (request: FastifyRequest, reply: FastifyReply) => {
    if (isPublicRoute(request.url)) return;

    // A supplied Authorization header always takes precedence, including an
    // empty or invalid header. It must never fall back to a development user.
    const authorization = request.headers.authorization;
    if (authorization === undefined) {
      if (allowDevAuth) {
        const header = !supabaseAdmin ? request.headers["x-user-id"] : undefined;
        const id = typeof header === "string" && header.length > 0 ? header : devUserId;
        if (id) {
          const user = await app.prisma.user.findUnique({ where: { id } });
          if (user) {
            request.userId = user.id;
            request.authId = user.authId;
            return;
          }
        }
      }
      return reply.status(401).send({ error: "Missing or invalid authorization header" });
    }

    const token = authorization.replace("Bearer ", "");
    if (!token || !supabaseAdmin) {
      return reply.status(401).send({ error: "Invalid or expired token" });
    }
    try {
      const { data, error } = await supabaseAdmin.auth.getUser(token);
      if (error || !data.user) {
        return reply.status(401).send({ error: "Invalid or expired token" });
      }
      const user = await app.prisma.user.upsert({
        where: { authId: data.user.id },
        update: { email: data.user.email || `user-${data.user.id}@supabase.local` },
        create: {
          authId: data.user.id,
          email: data.user.email || `user-${data.user.id}@supabase.local`,
        },
      });
      request.userId = user.id;
      request.authId = user.authId;
    } catch (err) {
      app.log.error(err);
      return reply.status(401).send({ error: "Authentication failed" });
    }
  });
});
