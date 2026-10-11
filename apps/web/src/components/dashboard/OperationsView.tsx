"use client";
import { useRef, useState, type FormEvent } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { usePrivateFetch } from "@/lib/private-session-context";
import type { ManualOperation, OperationLedger } from "@/types/operations-api";
import type { FinancialAccountDTO } from "@astra-wealth/shared/accounts";
import { useAccounts } from "./AccountsProvider";

const INPUT = "mt-1 w-full rounded-xl border border-outline-variant bg-surface-container-high px-3 py-2 focus-visible:outline-2 focus-visible:outline-primary";
const BUTTON = "rounded-full bg-primary px-5 py-2.5 text-body-sm font-semibold text-on-primary focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-primary disabled:opacity-50";
const LABELS: Record<string, string> = { BUY: "Compra", SELL: "Venda", INCOME: "Provento", FEE: "Taxa avulsa", TRANSFER: "Transferência interna", OPENING: "Saldo inicial do histórico" };
const CLASSES: Record<string, string> = { STOCK: "Ação", REAL_ESTATE_FUND: "FII", ETF: "ETF", FIXED_INCOME: "Renda fixa", CRYPTO: "Criptomoeda", MUTUAL_FUND: "Fundo", PENSION: "Previdência", OTHER: "Outro" };
const exact = (value: string | null | undefined) => value == null ? "Indisponível" : value.replace(".", ",");
function localTime(iso?: string) {
  const date = iso ? new Date(iso) : new Date();
  return new Date(date.getTime() - date.getTimezoneOffset() * 60_000).toISOString().slice(0, 19);
}
export function OperationsView({ account }: { account: FinancialAccountDTO }) {
  const fetchPrivate = usePrivateFetch();
  const client = useQueryClient();
  const { accounts, holdings, refresh } = useAccounts();
  const positions = holdings.filter(h => h.accountId === account.id && h.origin === "MANUAL");
  const path = `accounts/${encodeURIComponent(account.id)}/operations`;
  const query = useQuery({ queryKey: ["private", "operations", account.id], retry: false,
    queryFn: ({ signal }) => fetchPrivate<OperationLedger>(path, { signal }) });
  const [editing, setEditing] = useState<ManualOperation | null | undefined>();
  const [kind, setKind] = useState("BUY");
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  const [addAsset, setAddAsset] = useState(false);
  const request = useRef<{ signature: string; id: string } | null>(null);
  const busy = useRef(false);
  const formRef = useRef<HTMLFormElement>(null);
  const trade = ["BUY", "SELL", "OPENING"].includes(kind);
  const allowed = ["MANUAL", "BANK", "BROKERAGE"].includes(account.kind);
  const destinations = accounts.filter(a => a.id !== account.id && a.baseCurrency === account.baseCurrency && ["MANUAL", "BANK", "BROKERAGE"].includes(a.kind));
  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy.current) return;
    busy.current = true; setSaving(true); setError("");
    const form = new FormData(event.currentTarget);
    const string = (key: string) => String(form.get(key) ?? "").trim();
    const decimal = (key: string) => string(key).replace(",", ".") || null;
    try {
      const payload = {
        kind, currency: account.baseCurrency, occurredAt: new Date(string("occurredAt")).toISOString(),
        holdingId: trade || kind === "INCOME" ? string("holdingId") || null : null,
        market: trade ? string("market") : null, quantity: trade ? decimal("quantity") : null,
        destinationAccountId: kind === "TRANSFER" ? string("destinationAccountId") : null,
        amount: decimal("amount"), fees: ["BUY", "SELL", "INCOME"].includes(kind) ? decimal("fees") : null,
        note: string("note") || null, replacesId: editing?.id ?? null,
      };
      const signature = JSON.stringify(payload);
      if (request.current?.signature !== signature) request.current = { signature, id: crypto.randomUUID() };
      await fetchPrivate(path, { method: "POST", body: JSON.stringify({ ...payload, requestId: request.current.id }) });
      setEditing(undefined); request.current = null; refresh();
      await client.invalidateQueries({ queryKey: ["private", "operations"] });
    } catch (caught) { setError(caught instanceof Error ? caught.message : "Não foi possível salvar a operação."); }
    finally { setSaving(false); busy.current = false; }
  }
  async function createAsset(event: FormEvent<HTMLFormElement>) {
    event.preventDefault(); if (busy.current) return;
    busy.current = true; setSaving(true); setError("");
    const form = new FormData(event.currentTarget);
    try {
      await fetchPrivate(`accounts/${encodeURIComponent(account.id)}/holdings`, { method: "POST", body: JSON.stringify({
        name: String(form.get("name")).trim(), symbol: String(form.get("symbol") ?? "").trim() || null,
        assetClass: form.get("assetClass"), quantity: "0", currentValue: "0", unitPrice: null, dueDate: null,
      }) });
      setAddAsset(false); refresh();
    } catch (caught) { setError(caught instanceof Error ? caught.message : "Não foi possível cadastrar o ativo."); }
    finally { setSaving(false); busy.current = false; }
  }
  return <section className="rounded-3xl border border-outline-variant/30 bg-surface-container p-5 sm:p-6" aria-labelledby="operations-title">
    <div className="flex flex-wrap items-center justify-between gap-3">
      <h2 id="operations-title" className="text-title-md font-semibold">Operações manuais</h2>
      {allowed && <div className="flex flex-wrap gap-3">
        <button type="button" className={BUTTON} disabled={saving} onClick={() => { setEditing(null); setKind("BUY"); setError(""); }}>Registrar operação</button>
        <button type="button" className="rounded-full border border-outline-variant px-4 py-2 focus-visible:outline-primary" disabled={saving} onClick={() => setAddAsset(!addAsset)} aria-expanded={addAsset}>Cadastrar ativo</button>
      </div>}
    </div>
    <p className="mt-3 text-body-sm text-on-surface-variant">Histórico de operações e custo de aquisição. Estes registros não atualizam saldos ou avaliações patrimoniais. Aportes e retiradas ficam no formulário próprio. Transferências internas não são aportes. Compras, vendas e saldos iniciais tornam avaliações anteriores pendentes até atualização manual. Os cálculos não são apuração fiscal.</p>
    {!allowed && <p className="mt-3">Operações manuais disponíveis em contas manuais, bancos e corretoras.</p>}
    {error && <p role="alert" className="mt-3 text-error">{error}</p>}
    {addAsset && <form onSubmit={createAsset} className="mt-5 space-y-3 rounded-2xl border border-outline-variant p-4">
      <h3 className="font-semibold">Ativo para o histórico</h3>
      <p className="text-body-sm text-on-surface-variant">O cadastro começa com posição patrimonial zero, sem preço. Registre compras ou saldo inicial abaixo. A avaliação patrimonial é independente.</p>
      <label className="block">Nome<input name="name" required maxLength={200} className={INPUT} /></label>
      <label className="block">Ticker (opcional)<input name="symbol" maxLength={40} className={INPUT} /></label>
      <label className="block">Classe<select name="assetClass" className={INPUT}>{Object.entries(CLASSES).map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label>
      <button className={BUTTON} disabled={saving}>Salvar ativo</button>
    </form>}
    {editing !== undefined && <form ref={formRef} key={editing?.id ?? "new"} onSubmit={submit} className="mt-5 space-y-4 rounded-2xl border border-outline-variant p-4">
      <h3 className="font-semibold">{editing ? "Corrigir operação (original preservado)" : "Nova operação"}</h3>
      <fieldset disabled={saving} className="space-y-4">
        <label className="block">Tipo<select value={kind} onChange={event => setKind(event.target.value)} disabled={!!editing} className={INPUT}>{Object.entries(LABELS).map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label>
        {(trade || kind === "INCOME") && <label className="block">Ativo{kind === "INCOME" ? " (opcional)" : ""}<select name="holdingId" defaultValue={editing?.holdingId ?? ""} required={trade} className={INPUT} onChange={event => { if (editing && event.target.value !== editing.holdingId) event.target.value = editing.holdingId ?? ""; }}>
          <option value="">Selecione</option>{positions.filter(h => !editing || h.id === editing.holdingId).map(h => <option key={h.id} value={h.id}>{h.name}{h.symbol ? ` (${h.symbol})` : ""}</option>)}</select></label>}
        {trade && <>
          <label className="block">Mercado<select name="market" defaultValue={editing?.market ?? "UNKNOWN"} className={INPUT}>{(editing ? [editing.market ?? "UNKNOWN"] : ["UNKNOWN", "BR", "INTERNATIONAL"]).map(m => <option value={m} key={m}>{m === "BR" ? "Brasil" : m === "INTERNATIONAL" ? "Internacional" : "Não informado"}</option>)}</select></label>
          <p className="text-body-sm text-on-surface-variant">Custo médio apenas para ações, FIIs e ETFs brasileiros em BRL. Use saldo inicial antes da primeira venda se não houver compras no histórico.</p>
          <label className="block">Quantidade<input name="quantity" inputMode="decimal" required defaultValue={editing?.quantity ?? ""} className={INPUT} /></label>
        </>}
        {kind === "TRANSFER" && <label className="block">Conta de destino ({account.baseCurrency})<select name="destinationAccountId" required defaultValue={editing?.destinationAccountId ?? ""} className={INPUT}>
          <option value="">Selecione</option>{destinations.filter(a => !editing || a.id === editing.destinationAccountId).map(a => <option key={a.id} value={a.id}>{a.name}</option>)}</select></label>}
        <label className="block">{kind === "OPENING" ? "Custo total conhecido do saldo inicial" : "Valor bruto"} ({account.baseCurrency})<input name="amount" inputMode="decimal" required={!trade} defaultValue={editing?.amount ?? ""} className={INPUT} /></label>
        {["BUY", "SELL", "INCOME"].includes(kind) && <label className="block">Taxas da operação ({account.baseCurrency})<input name="fees" inputMode="decimal" defaultValue={editing?.fees ?? ""} className={INPUT} /></label>}
        <p className="text-body-sm text-on-surface-variant">Deixe valores desconhecidos em branco; informe 0 para confirmar ausência de taxa. Não registre novamente como taxa avulsa uma taxa incluída na compra, venda ou provento. Saldo inicial usa custo total já incluindo as taxas conhecidas.</p>
        <label className="block">Data e hora<input name="occurredAt" type="datetime-local" step="1" required max={localTime()} defaultValue={localTime(editing?.occurredAt)} className={INPUT} /></label>
        <label className="block">Observação<textarea name="note" maxLength={1000} defaultValue={editing?.note ?? ""} className={INPUT} /></label>
        <div className="flex gap-3"><button className={BUTTON}>{saving ? "Salvando…" : "Salvar operação"}</button><button type="button" onClick={() => { setEditing(undefined); setError(""); }} className="rounded-full border border-outline-variant px-4 py-2">Cancelar</button></div>
      </fieldset>
    </form>}
    {query.isPending ? <p role="status" className="mt-4">Carregando operações…</p> : query.error ? <div role="alert" className="mt-4">{query.error.message} <button onClick={() => void query.refetch()} className="text-primary">Tentar novamente</button></div> : <>
      {!!query.data?.positions.length && <div className="mt-6 overflow-x-auto"><h3 className="mb-2 font-semibold">Posições calculadas pelo histórico</h3><table className="w-full text-left text-body-sm"><thead><tr>{["Ativo", "Quantidade", `Custo acumulado (${account.baseCurrency})`, "Custo médio"].map(t => <th key={t} className="p-3">{t}</th>)}</tr></thead><tbody>{query.data.positions.map(p => <tr key={p.holdingId} className="border-t border-outline-variant/30"><td className="p-3">{positions.find(h => h.id === p.holdingId)?.name ?? p.holdingId}</td><td className="p-3 tabular-nums">{exact(p.quantity)}</td><td className="p-3 tabular-nums">{exact(p.accumulatedCost)}</td><td className="p-3 tabular-nums">{exact(p.averageCost)}</td></tr>)}</tbody></table></div>}
      {!query.data?.operations.length ? <p className="mt-5 text-on-surface-variant">Nenhuma operação registrada.</p> : <div className="mt-6 overflow-x-auto"><table className="w-full text-left text-body-sm"><caption className="mb-2 text-left font-semibold">Histórico e correções · {account.baseCurrency}</caption><thead><tr>{["Data / tipo", "Ativo / destino", "Quantidade", "Bruto / taxas", "Líquido", "Custo da venda", "Resultado realizado", "Registro"].map(t => <th className="p-3" key={t}>{t}</th>)}</tr></thead><tbody>{[...query.data.operations].reverse().map(op => {
        const result = query.data.results.find(r => r.id === op.id);
        return <tr key={op.id} className="border-t border-outline-variant/30 align-top"><td className="p-3 whitespace-nowrap">{new Date(op.occurredAt).toLocaleString("pt-BR")}<span className="block">{LABELS[op.kind]}{op.kind === "TRANSFER" ? op.accountId === account.id ? " · saída" : " · entrada" : ""}</span></td>
          <td className="p-3">{holdings.find(h => h.id === op.holdingId)?.name ?? (op.kind === "TRANSFER" ? accounts.find(a => a.id === (op.accountId === account.id ? op.destinationAccountId : op.accountId))?.name : "—")}</td>
          <td className="p-3 tabular-nums">{op.quantity ? exact(op.quantity) : "—"}</td><td className="p-3 tabular-nums">{exact(op.amount)}<span className="block text-on-surface-variant">Taxas: {["BUY", "SELL", "INCOME"].includes(op.kind) ? exact(op.fees) : "—"}</span></td>
          <td className="p-3 tabular-nums">{result?.active ? exact(result.netAmount) : "—"}</td><td className="p-3 tabular-nums">{result?.active && op.kind === "SELL" ? exact(result.assignedCost) : "—"}</td>
          <td className="p-3 tabular-nums">{result?.active && op.kind === "SELL" ? exact(result.realizedResult) : "—"}{result?.unavailableReason && <span className="block text-xs text-on-surface-variant">{result.unavailableReason === "UNSUPPORTED_COST_RULE" ? "Classe, mercado ou moeda sem regra definida" : "Custo, valor bruto ou taxa ausente"}</span>}</td>
          <td className="p-3"><span>{result?.active ? op.replacesId ? "Correção vigente" : "Vigente" : "Substituído"}</span>{op.note && <p className="max-w-64 break-words">{op.note}</p>}{result?.active && op.accountId === account.id && <button type="button" disabled={saving} className="mt-2 block text-primary focus-visible:outline-primary" onClick={() => { setEditing(op); setKind(op.kind); setError(""); request.current = null; requestAnimationFrame(() => formRef.current?.scrollIntoView({ block: "start" })); }}>Corrigir</button>}</td></tr>;
      })}</tbody></table></div>}
    </>}
  </section>;
}
