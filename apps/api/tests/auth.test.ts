import { afterEach, beforeEach, test, type TestContext } from "node:test";
import assert from "node:assert/strict";
import Fastify from "fastify";
import authPlugin from "../src/plugins/auth.js";
import meRoutes from "../src/modules/me/me.routes.js";
import { requireUserId } from "../src/lib/dev-auth.js";

const keys = ["NODE_ENV", "ALLOW_DEV_AUTH", "DEV_USER_ID", "SUPABASE_URL", "SUPABASE_SERVICE_ROLE_KEY"] as const;
let previous: Record<string, string | undefined>;
beforeEach(() => {
  previous = Object.fromEntries(keys.map(key => [key, process.env[key]]));
  for (const key of keys) delete process.env[key];
});
afterEach(() => {
  for (const key of keys) {
    if (previous[key] === undefined) delete process.env[key];
    else process.env[key] = previous[key];
  }
});
function configureSupabase() {
  process.env.SUPABASE_URL = "https://auth.test.invalid";
  process.env.SUPABASE_SERVICE_ROLE_KEY = "synthetic-service-key";
}
function fixture(t: TestContext, plugin = true) {
  const calls = { lookups: 0, identities: [] as string[], wallets: [] as string[], snapshots: [] as string[], watchlists: [] as string[] };
  const app = Fastify();
  app.decorate("prisma", {
    user: {
      findUnique: async ({ where }: { where: { id: string } }) => {
        calls.lookups++;
        return ["local-a", "local-b"].includes(where.id) ? { id: where.id, authId: `auth-${where.id}` } : null;
      },
      upsert: async ({ where }: { where: { authId: string } }) => {
        calls.identities.push(where.authId);
        assert.ok(["auth-a", "auth-b"].includes(where.authId));
        return { id: where.authId === "auth-a" ? "local-a" : "local-b", authId: where.authId };
      },
    },
    wallet: { findMany: async ({ where }: { where: { userId: string } }) => { calls.wallets.push(where.userId); return []; } },
    watchlist: { findMany: async ({ where }: { where: { userId: string } }) => { calls.watchlists.push(where.userId); return []; } },
    portfolioSnapshot: { findMany: async ({ where }: { where: { userId: string } }) => { calls.snapshots.push(where.userId); return []; } },
  });
  if (plugin) app.register(authPlugin);
  app.register(meRoutes);
  app.get("/v1/health", async () => ({ ok: true }));
  app.get("/v1/wallets-private", async (request, reply) => {
    const id = await requireUserId(request, reply);
    if (id) return { id };
  });
  t.after(() => app.close());
  return { app, calls };
}
function upstream(t: TestContext, fail = false) {
  const seen: string[] = [];
  t.mock.method(globalThis, "fetch", async (_url: unknown, options: RequestInit) => {
    const authorization = new Headers(options.headers).get("authorization");
    seen.push(authorization ?? "");
    if (fail) return Response.json({ message: "unavailable" }, { status: 503 });
    const identity = authorization === "Bearer fixture-a" ? "auth-a" : authorization === "Bearer fixture-b" ? "auth-b" : null;
    return identity ? Response.json({ id: identity, email: `${identity}@test.invalid`, aud: "authenticated", created_at: "2020-01-01T00:00:00Z", app_metadata: {}, user_metadata: {} })
      : Response.json({ message: "Invalid token" }, { status: 401 });
  });
  return seen;
}

test("missing or partial Supabase configuration fails closed outside explicit development", async (t) => {
  for (const environment of [undefined, "production", "staging", "test", "Development", "development"]) {
    for (const flag of [undefined, "false", "true"]) {
      if (environment === "development" && flag === "true") continue;
      for (const configured of ["none", "url-only", "key-only", "whitespace"]) {
        for (const key of keys) delete process.env[key];
        if (environment !== undefined) process.env.NODE_ENV = environment;
        if (flag !== undefined) process.env.ALLOW_DEV_AUTH = flag;
        process.env.DEV_USER_ID = "local-b";
        if (configured === "url-only") process.env.SUPABASE_URL = "https://auth.test.invalid";
        if (configured === "key-only") process.env.SUPABASE_SERVICE_ROLE_KEY = "synthetic";
        if (configured === "whitespace") process.env.SUPABASE_URL = "   ";
        const { app, calls } = fixture(t);
        await assert.rejects(app.ready(), /Authentication requires SUPABASE_URL/);
        assert.equal(calls.lookups, 0);
      }
    }
  }
});

test("explicit development permits only absent credentials and never partial configuration", async (t) => {
  process.env.NODE_ENV = "development";
  process.env.ALLOW_DEV_AUTH = "true";
  process.env.DEV_USER_ID = "local-a";
  const { app, calls } = fixture(t);
  await app.ready();
  assert.equal((await app.inject({ url: "/v1/me/wallets" })).statusCode, 200);
  assert.equal((await app.inject({ url: "/v1/me/wallets", headers: { "x-user-id": "local-b" } })).statusCode, 200);
  assert.deepEqual(calls.wallets, ["local-a", "local-b"]);
  const before = calls.lookups;
  for (const authorization of ["", "Bearer ", "Bearer broken", "broken"]) {
    assert.equal((await app.inject({ url: "/v1/me/wallets", headers: { authorization, "x-user-id": "local-b" } })).statusCode, 401);
  }
  assert.equal(calls.lookups, before);
  assert.equal((await app.inject({ url: "/v1/me/wallets", headers: { "x-user-id": "missing" } })).statusCode, 401);
  process.env.SUPABASE_URL = "https://auth.test.invalid";
  await assert.rejects(fixture(t).app.ready(), /Authentication requires SUPABASE_URL/);
  delete process.env.SUPABASE_URL;
  process.env.SUPABASE_SERVICE_ROLE_KEY = "synthetic";
  await assert.rejects(fixture(t).app.ready(), /Authentication requires SUPABASE_URL/);
});

test("all legacy private routes reject header-only and development identities in production", async (t) => {
  configureSupabase();
  process.env.NODE_ENV = "production";
  process.env.ALLOW_DEV_AUTH = "true";
  process.env.DEV_USER_ID = "local-b";
  const { app, calls } = fixture(t);
  for (const [method, url] of [
    ["GET", "/v1/me/wallets"], ["POST", "/v1/me/wallets"], ["DELETE", "/v1/me/wallets/w"], ["POST", "/v1/me/wallets/w/sync"],
    ["GET", "/v1/me/watchlists"], ["POST", "/v1/me/watchlists"], ["DELETE", "/v1/me/watchlists/w"],
    ["POST", "/v1/me/watchlists/w/items"], ["DELETE", "/v1/me/watchlists/w/items/coin"],
    ["GET", "/v1/me/portfolio/snapshots"], ["POST", "/v1/me/portfolio/snapshots"], ["GET", "/v1/wallets-private"],
  ] as const) {
    assert.equal((await app.inject({ method, url, headers: { "x-user-id": "local-b" } })).statusCode, 401, `${method} ${url}`);
  }
  assert.deepEqual(calls, { lookups: 0, identities: [], wallets: [], snapshots: [], watchlists: [] });
  assert.equal((await app.inject({ url: "/v1/health?probe=1" })).statusCode, 200);
});

test("verified A and B identities scope real me handlers and override forged owner headers", async (t) => {
  configureSupabase();
  process.env.NODE_ENV = "production";
  process.env.DEV_USER_ID = "local-b";
  upstream(t);
  const { app, calls } = fixture(t);
  for (const token of ["fixture-a", "fixture-b"]) {
    for (const url of ["/v1/me/wallets", "/v1/me/watchlists", "/v1/me/portfolio/snapshots"]) {
      assert.equal((await app.inject({ url, headers: { authorization: `Bearer ${token}`, "x-user-id": "local-b" } })).statusCode, 200);
    }
  }
  assert.deepEqual(calls.wallets, ["local-a", "local-b"]);
  assert.deepEqual(calls.watchlists, ["local-a", "local-b"]);
  assert.deepEqual(calls.snapshots, ["local-a", "local-b"]);
  assert.equal(calls.lookups, 0);
});

test("invalid credentials never fall back even with configured Supabase and explicit development", async (t) => {
  configureSupabase();
  process.env.NODE_ENV = "development";
  process.env.ALLOW_DEV_AUTH = "true";
  process.env.DEV_USER_ID = "local-b";
  upstream(t);
  const { app, calls } = fixture(t);
  for (const authorization of ["", "Bearer ", "Bearer broken"]) {
    assert.equal((await app.inject({ url: "/v1/me/wallets", headers: { authorization, "x-user-id": "local-b" } })).statusCode, 401);
  }
  assert.equal(calls.lookups, 0);
  assert.deepEqual(calls.identities, []);
  assert.equal((await app.inject({ url: "/v1/me/wallets", headers: { authorization: "Bearer fixture-a", "x-user-id": "local-b" } })).statusCode, 200);
  assert.deepEqual(calls.wallets, ["local-a"]);
});

test("legacy helper cannot authenticate without the plugin, even in explicit development", async (t) => {
  process.env.NODE_ENV = "development";
  process.env.ALLOW_DEV_AUTH = "true";
  process.env.DEV_USER_ID = "local-b";
  const { app, calls } = fixture(t, false);
  assert.equal((await app.inject({ url: "/v1/me/wallets", headers: { "x-user-id": "local-b" } })).statusCode, 401);
  assert.equal(calls.lookups, 0);
});
