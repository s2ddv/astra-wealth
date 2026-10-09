import { test } from "node:test";
import assert from "node:assert/strict";
import { MutationObserver } from "@tanstack/react-query";
import { createPrivateSessionCache } from "../src/lib/private-session.ts";

const deferred = () => Promise.withResolvers();
const aborted = (error) => error?.name === "AbortError";
const accounts = ["private", "accounts"];

test("initialization, logout and account changes isolate caches; token refresh preserves them", async (t) => {
  const sessions = createPrivateSessionCache();
  t.after(() => sessions.dispose());
  const transport = t.mock.method(globalThis, "fetch", async () => Response.json([]));
  await assert.rejects(sessions.current.fetch("accounts"), { status: 401 });
  const a = sessions.switchTo("user-a");
  a.client.setQueryData(accounts, [{ id: "account-a" }]);
  assert.equal(sessions.switchTo("user-a"), a); // repeated SIGNED_IN / TOKEN_REFRESHED
  const b = sessions.switchTo("user-b");
  assert.notEqual(b.client, a.client);
  assert.equal(a.client.getQueryData(accounts), undefined);
  assert.equal(b.client.getQueryData(accounts), undefined);
  await assert.rejects(a.fetch("accounts", { method: "POST" }), aborted);
  const signedOut = sessions.switchTo(null);
  await assert.rejects(signedOut.fetch("accounts"), { status: 401 });
  const returned = sessions.switchTo("user-a");
  assert.notEqual(returned.client, a.client);
  assert.equal(returned.client.getQueryData(accounts), undefined);
  assert.ok(returned.generation > b.generation);
  sessions.dispose();
  const remounted = sessions.switchTo("user-a"); // effect cleanup/setup in Strict Mode
  assert.notEqual(remounted.client, returned.client);
  assert.equal(remounted.disposed, false);
  assert.equal(transport.mock.callCount(), 0);
});

test("a late mutation from A cannot populate B or a later session of A", async (t) => {
  const sessions = createPrivateSessionCache();
  t.after(() => sessions.dispose());
  const a = sessions.switchTo("user-a");
  const started = deferred();
  const response = deferred();
  let requestSignal;
  t.mock.method(globalThis, "fetch", (_url, options) => {
    requestSignal = options.signal;
    started.resolve();
    return response.promise; // deliberately ignores abort to reproduce a late response
  });
  let successes = 0;
  const mutation = new MutationObserver(a.client, {
    mutationFn: () => a.fetch("accounts", { method: "POST" }),
    onSuccess: (account) => { successes++; a.client.setQueryData(accounts, [account]); },
  });
  const result = assert.rejects(mutation.mutate(), aborted);
  await started.promise;
  const b = sessions.switchTo("user-b");
  b.client.setQueryData(accounts, [{ id: "account-b" }]);
  assert.equal(requestSignal.aborted, true);
  response.resolve(Response.json({ id: "account-a" }));
  await result;
  assert.equal(successes, 0);
  assert.deepEqual(b.client.getQueryData(accounts), [{ id: "account-b" }]);
  assert.equal(sessions.switchTo("user-a").client.getQueryData(accounts), undefined);
});

test("even an uncancellable old callback retains only its retired QueryClient", async (t) => {
  const sessions = createPrivateSessionCache();
  t.after(() => sessions.dispose());
  const a = sessions.switchTo("user-a");
  const response = deferred();
  const mutation = new MutationObserver(a.client, {
    mutationFn: () => response.promise,
    onSuccess: (value) => a.client.setQueryData(accounts, value),
  });
  const pending = mutation.mutate();
  const b = sessions.switchTo("user-b");
  b.client.setQueryData(accounts, ["account-b"]);
  const returnedA = sessions.switchTo("user-a");
  response.resolve(["late-account-a"]);
  await pending;
  assert.deepEqual(b.client.getQueryData(accounts), undefined); // also retired
  assert.deepEqual(a.client.getQueryData(accounts), ["late-account-a"]);
  assert.equal(returnedA.client.getQueryData(accounts), undefined);
  a.client.clear();
});

test("retiring a session cancels queries and rejects late body parsing and 204 responses", async (t) => {
  for (const status of [200, 204]) {
    const sessions = createPrivateSessionCache();
    t.after(() => sessions.dispose());
    const a = sessions.switchTo("user-a");
    const started = deferred();
    const body = deferred();
    const mock = t.mock.method(globalThis, "fetch", async () => {
      if (status === 204) { started.resolve(); await body.promise; return new Response(null, { status }); }
      return { status, ok: true, json: () => { started.resolve(); return body.promise; } };
    });
    const pending = a.client.fetchQuery({ queryKey: accounts, queryFn: ({ signal }) => a.fetch("accounts", { signal }) });
    const rejected = assert.rejects(pending);
    await started.promise;
    const b = sessions.switchTo("user-b");
    body.resolve([{ id: "account-a" }]);
    await rejected;
    assert.equal(b.client.getQueryData(accounts), undefined);
    assert.equal(a.client.getQueryCache().getAll().length, 0);
    mock.mock.restore();
  }
});

test("multi-step work and queued mutations cannot send another request after an identity switch", async (t) => {
  const sessions = createPrivateSessionCache();
  t.after(() => sessions.dispose());
  const a = sessions.switchTo("user-a");
  const firstDone = deferred();
  const resume = deferred();
  const transport = t.mock.method(globalThis, "fetch", async () => Response.json({ id: "list-a" }));
  const mutation = new MutationObserver(a.client, {
    mutationFn: async () => {
      const list = await a.fetch("watchlists", { method: "POST" });
      firstDone.resolve();
      await resume.promise;
      return a.fetch(`watchlists/${list.id}/items`, { method: "POST" });
    },
  });
  const result = assert.rejects(mutation.mutate(), aborted);
  await firstDone.promise;
  sessions.switchTo("user-b");
  resume.resolve();
  await result;
  await assert.rejects(a.fetch("wallets/wallet-a/sync", { method: "POST" }), aborted);
  assert.equal(transport.mock.callCount(), 1);
});

test("query cancellation composes with session cancellation without disabling the next user's requests", async (t) => {
  const sessions = createPrivateSessionCache();
  t.after(() => sessions.dispose());
  const a = sessions.switchTo("user-a");
  const query = new AbortController();
  query.abort();
  const transport = t.mock.method(globalThis, "fetch", async (_url, options) => {
    assert.equal(options.signal.aborted, false);
    return Response.json([{ id: "account-b" }]);
  });
  await assert.rejects(a.fetch("accounts", { signal: query.signal }), aborted);
  const b = sessions.switchTo("user-b");
  assert.deepEqual(await b.fetch("accounts"), [{ id: "account-b" }]);
  assert.equal(transport.mock.callCount(), 1);
});
