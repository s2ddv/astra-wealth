// Round only for display; never convert API decimal strings through Number.
export function portfolioMoney(value: string | null): string {
  if (value === null || !/^-?\d+(\.\d+)?$/.test(value)) return "Indisponível";
  const negative = value.startsWith("-");
  const [whole = "0", fraction = ""] = (negative ? value.slice(1) : value).split(".");
  let cents = BigInt(whole) * 100n + BigInt((fraction + "00").slice(0, 2));
  if ((fraction[2] ?? "0") >= "5") cents += 1n;
  const integer = new Intl.NumberFormat("pt-BR").format(cents / 100n);
  return `${negative && cents !== 0n ? "−" : ""}R$ ${integer},${(cents % 100n).toString().padStart(2, "0")}`;
}
export function portfolioTitle(complete: boolean, knownValue: string | null): string {
  return knownValue === null ? "Patrimônio sem cotação" : complete ? "Patrimônio total" : "Subtotal conhecido · parcial";
}
