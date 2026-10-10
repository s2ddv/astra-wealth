"use client";
import Link from "next/link";
import { useQuery, useMutation } from "@tanstack/react-query";
import { usePrivateFetch } from "@/lib/private-session-context";
import { portfolioMoney, portfolioTitle } from "@/lib/portfolio-format";
import type { PortfolioSnapshot } from "@/types/portfolio-api";
import styles from "./Overview.module.css";

const issues: Record<string, string> = {
  NOT_SYNCED: "Carteira ainda não sincronizada", PARTIAL_BALANCES: "Sincronização parcial",
  STALE_BALANCE: "Saldo desatualizado", STALE_FX: "PTAX em cache; atualização indisponível",
  MISSING_QUOTE: "Cotação indisponível", UNSUPPORTED_ASSET: "Ativo ainda sem suporte à avaliação",
};
const dateTime = (value: string) => new Intl.DateTimeFormat("pt-BR", { dateStyle: "short", timeStyle: "short", timeZone: "America/Sao_Paulo" }).format(new Date(value));
export function RealPortfolioOverview() {
  const privateFetch = usePrivateFetch();
  const query = useQuery({ queryKey: ["private", "portfolio"], retry: false, staleTime: 60_000,
    queryFn: ({ signal }) => privateFetch<PortfolioSnapshot>("portfolio", { signal }),
  });
  const capture = useMutation({ mutationFn: () => privateFetch<{ id: string; snapshot: PortfolioSnapshot }>("portfolio/snapshots", { method: "POST", body: "{}" }) });
  if (query.isPending) return <p role="status">Carregando patrimônio…</p>;
  if (query.isError) return <section role="alert"><p>{query.error.message}</p><button onClick={() => void query.refetch()} disabled={query.isFetching}>Tentar novamente</button></section>;
  const data = query.data;
  return <div className={styles.overview}>
    <header className={styles.heading}><div><p className={styles.eyebrow}>Visão geral</p><h1>Seu patrimônio</h1>
      <p className={styles.muted}>Contas registradas e carteiras sincronizadas, em reais.</p></div>
      <button onClick={() => void query.refetch()} disabled={query.isFetching}>{query.isFetching ? "Atualizando…" : "Atualizar cotações"}</button>
    </header>
    <section className={styles.card} aria-labelledby="real-portfolio-title">
      <h2 id="real-portfolio-title">{portfolioTitle(data.complete, data.knownValueBrl)}</h2>
      <p className={styles.total}>{portfolioMoney(data.knownValueBrl)}</p>
      {!data.complete && <p role="status">Há posições sem cotação, incompletas ou desatualizadas. Este subtotal não representa seu patrimônio completo.</p>}
      {data.positions.length === 0 && <p>Nenhuma posição registrada. <Link href="/dashboard/accounts">Adicionar uma conta</Link> ou <Link href="/dashboard/wallets">acompanhar uma carteira</Link>.</p>}
      <dl className={styles.metrics}>
        <div><dt>Aportes líquidos</dt><dd>{portfolioMoney(data.netContributionsBrl)}</dd></div>
        <div><dt>Resultado acumulado</dt><dd>{portfolioMoney(data.resultBrl)}</dd></div>
      </dl>
      {!data.contributionsComplete && <p role="status">Há aportes sem taxa histórica. O resultado ficará indisponível até que sejam cotados.</p>}
      <p className={styles.explanation}>Resultado = patrimônio atual − aportes líquidos registrados. Transferências e sincronizações não são aportes; aportes não registrados tornam essa comparação incompleta.</p>
      <p className={styles.muted}>Posições consultadas em {dateTime(data.observedAt)}. Avaliação em {dateTime(data.valuedAt)} (Brasília).</p>
      <button onClick={() => capture.mutate()} disabled={capture.isPending}>{capture.isPending ? "Salvando…" : "Salvar avaliação no histórico"}</button>
      {capture.isSuccess && <p role="status">Avaliação salva com os valores e as taxas usados naquele momento.</p>}
      {capture.isError && <p role="alert">Não foi possível salvar. {capture.error.message}</p>}
    </section>
    <section className={styles.card} aria-labelledby="valuation-sources"><h2 id="valuation-sources">Posições e fontes</h2>
      <ul>{data.positions.map((p) => <li key={p.id}>
        <strong>{p.name}</strong>: {portfolioMoney(p.valueBrl)}
        <p className={styles.muted}>{p.source === "COINGECKO_BRL" ? "CoinGecko · preço em BRL" : p.source === "RECORDED_HOLDING" ? "Valor registrado na conta" : p.source === "SYNCED_BALANCE" ? "Saldo sincronizado" : "Sem avaliação"}
          {p.priceFetchedAt && ` · consulta em ${dateTime(p.priceFetchedAt)}`} · saldo de {dateTime(p.balanceUpdatedAt)}</p>
        {p.fx?.quote && <p>BCB PTAX · venda/fechamento · US$ 1 = R$ {p.fx.quote.rate} · data da taxa: {p.fx.quote.effectiveDate}
          {p.fx.quote.effectiveDate !== p.fx.requestedDate && " · último fechamento anterior à data solicitada"}</p>}
        {p.issue && <p role="status">{issues[p.issue] ?? "Avaliação parcial"}. {p.walletId && <Link href="/dashboard/wallets">Consultar carteira</Link>}</p>}
      </li>)}</ul>
    </section>
    {data.contributions.length > 0 && <details className={styles.card}><summary>Taxas dos aportes</summary><ul>{data.contributions.map((c) => <li key={c.id}>
      {c.occurredOn} · {portfolioMoney(c.valueBrl)} · {c.source === "RECORDED" ? `Taxa informada preservada: ${c.rate}` : c.source === "BRL" ? "Aporte em reais" : `BCB PTAX: ${c.rate ?? "indisponível"}`}
      {c.fx?.quote && ` · data da taxa: ${c.fx.quote.effectiveDate}`}
    </li>)}</ul></details>}
  </div>;
}
