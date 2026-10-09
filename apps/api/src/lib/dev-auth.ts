import type { FastifyReply, FastifyRequest } from "fastify";

/** Require the local identity established by the authentication plugin. */
export async function requireUserId(
  request: FastifyRequest,
  reply: FastifyReply,
): Promise<string | null> {
  if (request.userId) return request.userId;
  reply.status(401).send({ error: "Authentication required" });
  return null;
}
