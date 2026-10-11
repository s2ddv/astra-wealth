import { test } from "node:test";
import assert from "node:assert/strict";
import { privateRoute } from "../src/lib/private-routes.ts";
import { privateFetch, PrivateApiError } from "../src/lib/private-api.ts";

test("domain bridge allows only implemented Rust method/path pairs", () => {
  assert.equal(privateRoute("wallets", [], "GET"), "/v1/me/wallets");
  assert.equal(privateRoute("wallets", ["w1", "sync"], "POST"), "/v1/me/wallets/w1/sync");
  assert.equal(privateRoute("accounts", ["a1", "operations"], "GET"), "/v1/accounts/a1/operations");
  assert.equal(privateRoute("accounts", ["a1", "operations"], "POST"), "/v1/accounts/a1/operations");
  assert.equal(privateRoute("accounts", ["a1", "contributions"], "POST"), "/v1/accounts/a1/contributions");
  assert.equal(privateRoute("watchlists", ["w1", "items", "crypto:bitcoin"], "DELETE"), "/v1/me/watchlists/w1/items/crypto%3Abitcoin");
  for (const [domain, path, method] of [
    ["accounts", ["a1", "operations"], "DELETE"],
    ["accounts", ["a1", "operations", "o1"], "PUT"],
    ["admin", [], "GET"], ["wallets", ["w1", "sync"], "GET"],
    ["accounts", ["a1", "contributions", "c1"], "PUT"],
    ["wallets", [".."], "DELETE"], ["wallets", ["%2e%2e"], "DELETE"],
    ["wallets", ["a/b"], "DELETE"], ["wallets", ["a?admin=true"], "DELETE"],
  ]) assert.equal(privateRoute(domain, path, method), null);
});

test("client preserves decimal strings and handles empty DELETE responses", async (t) => {
  t.mock.method(globalThis, "fetch", async (url, options) => {
    assert.equal(options.credentials, "same-origin");
    assert.equal(options.cache, "no-store");
    return options.method === "DELETE" ? new Response(null, { status: 204 })
      : Response.json({ amount: "1.000000000000000001", priceUsd: null, complete: false });
  });
  assert.deepEqual(await privateFetch("wallets/w1/assets"), { amount: "1.000000000000000001", priceUsd: null, complete: false });
  assert.equal(await privateFetch("wallets/w1", { method: "DELETE" }), undefined);
});

test("failed mutations surface authentication, rate and ownership errors", async (t) => {
  for (const [status, code, text] of [[401, null, "Entre"], [429, "RATE_LIMITED", "Aguarde"], [409, "WALLET_LINKED_TO_ACCOUNT", "vinculada"]]) {
    const mock = t.mock.method(globalThis, "fetch", async () => Response.json({ code }, { status }));
    await assert.rejects(privateFetch("wallets/w1/sync", { method: "POST" }), (error) => error instanceof PrivateApiError && error.status === status && error.message.includes(text));
    mock.mock.restore();
  }
});
