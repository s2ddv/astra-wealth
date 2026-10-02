import { test } from "node:test";
import assert from "node:assert/strict";
import {
  calculateContributionsSummary as calculate,
  createContributionSchema,
  type ContributionDTO,
} from "../src/accounts.ts";
const deposit = (
  amount: string,
): Pick<ContributionDTO, "kind" | "amount" | "currency" | "fxRateToBrl"> => ({
  kind: "DEPOSIT",
  amount,
  currency: "BRL",
  fxRateToBrl: null,
});
test("soma depósitos sem perda decimal", () => {
  const summary = calculate([deposit("0.1"), deposit("0.2")], "0.6");
  assert.equal(summary.netContributed, "0.3");
  assert.equal(summary.result, "0.3");
  assert.equal(summary.resultPct, "1");
});
test("desconta retiradas", () => {
  const summary = calculate(
    [deposit("100"), { ...deposit("20"), kind: "WITHDRAWAL" }],
    "120",
  );
  assert.equal(summary.totalContributed, "100");
  assert.equal(summary.totalWithdrawn, "20");
  assert.equal(summary.netContributed, "80");
  assert.equal(summary.resultPct, "0.5");
});
test("base zero ou negativa não tem rentabilidade", () => {
  assert.equal(calculate([], "0").resultPct, null);
  assert.equal(
    calculate([deposit("10"), { ...deposit("10"), kind: "WITHDRAWAL" }], "0")
      .resultPct,
    null,
  );
  assert.equal(
    calculate([{ ...deposit("20"), kind: "WITHDRAWAL" }], "0").resultPct,
    null,
  );
});
test("converte USD pela cotação histórica", () => {
  assert.equal(
    calculate(
      [{ ...deposit("100"), currency: "USD", fxRateToBrl: "5.25" }],
      "600",
    ).netContributed,
    "525",
  );
  assert.throws(() =>
    calculate([{ ...deposit("100"), currency: "USD" }], "600"),
  );
});
test("preserva precisão em valores grandes", () => {
  assert.equal(
    calculate(
      [deposit("999999999999999999.1234567891")],
      "999999999999999999.1234567892",
    ).result,
    "0.0000000001",
  );
});
test("validação rejeita valores inválidos, data futura e conta vazia", () => {
  const input = {
    accountId: "conta",
    kind: "DEPOSIT",
    currency: "BRL",
    amount: "10.25",
    occurredAt: "2026-01-01T12:00:00.000Z",
  };
  assert.equal(createContributionSchema.safeParse(input).success, true);
  for (const change of [
    { amount: "0" },
    { amount: "-1" },
    { amount: "NaN" },
    { accountId: " " },
    { occurredAt: "2099-01-01T00:00:00.000Z" },
    { occurredAt: "inválida" },
    { currency: "USD" },
  ])
    assert.equal(
      createContributionSchema.safeParse({ ...input, ...change }).success,
      false,
    );
});

test("resultado negativo e câmbio distinto na retirada", () => {
  const summary = calculate([
    { ...deposit("100"), currency: "USD", fxRateToBrl: "5" },
    { ...deposit("20"), kind: "WITHDRAWAL", currency: "USD", fxRateToBrl: "6" },
  ], "342");
  assert.equal(summary.totalContributed, "500");
  assert.equal(summary.totalWithdrawn, "120");
  assert.equal(summary.netContributed, "380");
  assert.equal(summary.result, "-38");
  assert.equal(summary.resultPct, "-0.1");
});
