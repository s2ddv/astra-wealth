import { test } from "node:test";
import assert from "node:assert/strict";
import { portfolioMoney, portfolioTitle } from "../src/lib/portfolio-format.ts";
import { privateRoute } from "../src/lib/private-routes.ts";
test("display keeps large exact amounts and distinguishes unknown from zero", () => {
  assert.equal(portfolioMoney("999999999999999999.995"), "R$ 1.000.000.000.000.000.000,00");
  assert.equal(portfolioMoney("-0.005"), "−R$ 0,01");
  assert.equal(portfolioMoney("0"), "R$ 0,00");
  assert.equal(portfolioMoney(null), "Indisponível");
  assert.equal(portfolioTitle(false, "0"), "Subtotal conhecido · parcial");
  assert.equal(portfolioTitle(false, null), "Patrimônio sem cotação");
});
test("portfolio bridge restricts methods, snapshots and historical date routes", () => {
  assert.equal(privateRoute("portfolio", [], "GET"), "/v1/me/portfolio");
  assert.equal(privateRoute("portfolio", ["snapshots"], "POST"), "/v1/me/portfolio/snapshots");
  assert.equal(privateRoute("fx", ["usd-brl", "2024-01-07"], "GET"), "/v1/me/fx/usd-brl/2024-01-07");
  for (const [domain, path, method] of [["portfolio",[],"POST"],["portfolio",["other-user"],"GET"],["portfolio",["snapshots"],"DELETE"],["fx",["eur-brl"],"GET"],["fx",["usd-brl","yesterday"],"GET"]]) assert.equal(privateRoute(domain,path,method),null);
});
