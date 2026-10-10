import { test } from "node:test";
import assert from "node:assert/strict";
import { createHmac } from "node:crypto";
import Fastify from "fastify";
import { PrismaClient } from "@astra-wealth/database";
import authPlugin from "../src/plugins/auth.js";
import meRoutes from "../src/modules/me/me.routes.js";
import { WalletService } from "../src/modules/wallet/wallet.service.js";

// Intentionally excluded from the fast unit suite. The security gate runs it
// explicitly after Rust against the same freshly created disposable database.
test("Fastify preserves Rust identities and isolates every legacy private domain", async (t) => {
  const url = new URL(process.env.TEST_DATABASE_URL ?? "http://invalid");
  assert.equal(url.hostname, "127.0.0.1", "Use npm run test:security");
  assert.equal(url.pathname, "/astra_isolation_test");
  const db = new PrismaClient({ datasources: { db: { url: url.toString() } } });
  t.after(() => db.$disconnect());
  const subjects = ["isolation-auth-a", "isolation-auth-b"];
  const owners = ["isolation-local-a", "isolation-local-b"];
  const beforeUsers = await db.user.findMany({ orderBy: { id: "asc" }, select: { id: true, authId: true } });
  assert.deepEqual(beforeUsers, owners.map((id, i) => ({ id, authId: subjects[i] })));

  const settingKeys = ["NODE_ENV", "SUPABASE_URL", "SUPABASE_SERVICE_ROLE_KEY", "ALLOW_DEV_AUTH", "DEV_USER_ID"] as const;
  const previous = Object.fromEntries(settingKeys.map(key => [key, process.env[key]]));
  t.after(() => {
    for (const key of settingKeys) {
      if (previous[key] === undefined) delete process.env[key]; else process.env[key] = previous[key];
    }
  });
  process.env.NODE_ENV = "production";
  process.env.SUPABASE_URL = "https://isolation.test.invalid";
  process.env.SUPABASE_SERVICE_ROLE_KEY = "synthetic-service-key";
  process.env.ALLOW_DEV_AUTH = "true";
  process.env.DEV_USER_ID = owners[1];
  const encode = (value: unknown) => Buffer.from(JSON.stringify(value)).toString("base64url");
  const tokens = subjects.map(sub => {
    const payload = `${encode({ alg: "HS256", typ: "JWT" })}.${encode({ sub, aud: "authenticated", exp: Math.floor(Date.now() / 1000) + 3600 })}`;
    return `${payload}.${createHmac("sha256", "synthetic-isolation-signing-secret").update(payload).digest("base64url")}`;
  });
  t.mock.method(globalThis, "fetch", async (target: unknown, options: RequestInit) => {
    assert.equal(String(target), "https://isolation.test.invalid/auth/v1/user");
    const actor = tokens.findIndex(token => new Headers(options.headers).get("authorization") === `Bearer ${token}`);
    return actor < 0 ? Response.json({ message: "Invalid token" }, { status: 401 })
      : Response.json({ id: subjects[actor], email: `${subjects[actor]}@test.invalid`, aud: "authenticated", app_metadata: {}, user_metadata: {} });
  });
  const provider = t.mock.method(WalletService.prototype, "getSummary", async (address: string) => ({
    address, alchemy: null, moralis: null, fetchedAt: "2020-01-01T00:00:00Z",
  }));
  const app = Fastify();
  app.decorate("prisma", db);
  app.register(authPlugin);
  app.register(meRoutes);
  t.after(() => app.close());
  async function request(actor: number, method: "GET" | "POST" | "DELETE", path: string, payload: unknown, status: number) {
    const response = await app.inject({ method, url: path,
      headers: { authorization: `Bearer ${tokens[actor]}`, "x-user-id": owners[1 - actor]! },
      ...(payload === undefined ? {} : { payload: payload as Record<string, unknown> }),
    });
    assert.equal(response.statusCode, status, `${method} ${path}: ${response.body}`);
    return status === 204 ? undefined : response.json();
  }
  const wallets = [];
  const lists = [];
  const snapshots = [];
  for (let actor = 0; actor < 2; actor++) {
    const existing = await request(actor, "GET", "/v1/me/wallets", undefined, 200);
    assert.equal(existing.length, 1);
    assert.equal(existing[0].userId, owners[actor]);
    assert.ok(existing[0].assets.length > 0, "Fastify must see only its owner's Rust assets");
    wallets.push(await request(actor, "POST", "/v1/me/wallets", { address: "0x0000000000000000000000000000000000000002", chain: "ETHEREUM", nickname: `Legacy ${actor}`, userId: owners[1 - actor] }, 201));
    lists.push(await request(actor, "POST", "/v1/me/watchlists", { name: "Legacy", userId: owners[1 - actor] }, 201));
    await request(actor, "POST", `/v1/me/watchlists/${lists[actor].id}/items`, { coinId: `asset-${actor}` }, 201);
    snapshots.push(await request(actor, "POST", "/v1/me/portfolio/snapshots", { totalUsd: 100 + actor, userId: owners[1 - actor] }, 201));
    assert.equal(wallets[actor].userId, owners[actor]);
    assert.equal(lists[actor].userId, owners[actor]);
    assert.equal(snapshots[actor].userId, owners[actor]);
  }
  const beforeWallets = await db.wallet.findMany({ orderBy: { id: "asc" }, include: { assets: { orderBy: { id: "asc" } } } });
  const beforeLists = await db.watchlist.findMany({ orderBy: { id: "asc" }, include: { items: { orderBy: { id: "asc" } } } });
  const beforeSnapshots = await db.portfolioSnapshot.findMany({ orderBy: { id: "asc" } });
  const providerCalls = provider.mock.callCount();
  for (let actor = 0; actor < 2; actor++) {
    const other = 1 - actor;
    for (const [method, path, payload] of [
      ["DELETE", `/v1/me/wallets/${wallets[other].id}`, undefined],
      ["POST", `/v1/me/wallets/${wallets[other].id}/sync`, {}],
      ["DELETE", `/v1/me/watchlists/${lists[other].id}`, undefined],
      ["POST", `/v1/me/watchlists/${lists[other].id}/items`, { coinId: "intruder" }],
      ["DELETE", `/v1/me/watchlists/${lists[other].id}/items/asset-${other}`, undefined],
      ["DELETE", `/v1/me/watchlists/${lists[actor].id}/items/asset-${other}`, undefined],
    ] as const) await request(actor, method, path, payload, 404);
    const ownWallets = await request(actor, "GET", "/v1/me/wallets", undefined, 200);
    const ownLists = await request(actor, "GET", "/v1/me/watchlists", undefined, 200);
    const ownSnapshots = await request(actor, "GET", "/v1/me/portfolio/snapshots", undefined, 200);
    for (const rows of [ownWallets, ownLists, ownSnapshots]) {
      assert.ok(rows.length > 0);
      assert.ok(rows.every((row: { userId: string }) => row.userId === owners[actor]));
    }
    assert.equal(ownSnapshots.length, 1);
    assert.equal(ownSnapshots[0].id, snapshots[actor].id);
  }
  assert.equal(provider.mock.callCount(), providerCalls, "Cross-user sync reached a provider");
  assert.deepEqual(await db.wallet.findMany({ orderBy: { id: "asc" }, include: { assets: { orderBy: { id: "asc" } } } }), beforeWallets);
  assert.deepEqual(await db.watchlist.findMany({ orderBy: { id: "asc" }, include: { items: { orderBy: { id: "asc" } } } }), beforeLists);
  assert.deepEqual(await db.portfolioSnapshot.findMany({ orderBy: { id: "asc" } }), beforeSnapshots);
  assert.deepEqual(await db.user.findMany({ orderBy: { id: "asc" }, select: { id: true, authId: true } }), beforeUsers);
  for (let actor = 0; actor < 2; actor++) {
    await request(actor, "POST", `/v1/me/wallets/${wallets[actor].id}/sync`, {}, 200);
    await request(actor, "DELETE", `/v1/me/watchlists/${lists[actor].id}/items/asset-${actor}`, undefined, 204);
    await request(actor, "DELETE", `/v1/me/watchlists/${lists[actor].id}`, undefined, 204);
    await request(actor, "DELETE", `/v1/me/wallets/${wallets[actor].id}`, undefined, 204);
  }
});
