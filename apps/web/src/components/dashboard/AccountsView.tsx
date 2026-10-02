"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import {
  useEffect,
  useRef,
  useState,
  type ReactNode,
  type FormEvent,
} from "react";
import {
  createContributionSchema,
  type AccountKind,
  type FinancialAccountDTO,
} from "@astra-wealth/shared/accounts";
import {
  mockCryptoPositions,
  mockHoldings,
  summarizeAccounts,
} from "@/mocks/accounts";
import { useAccounts } from "./AccountsProvider";
import {
  ACCOUNT_ICONS,
  ACCOUNT_LABELS,
  ContributionsKpis,
  money,
  percent,
  syncLabel,
} from "./AccountPresentation";

const INPUT =
  "mt-1 w-full rounded-xl border border-outline-variant bg-surface-container-high px-3 py-2 focus-visible:outline-2 focus-visible:outline-primary";
const BUTTON =
  "rounded-full bg-primary px-5 py-2.5 text-body-sm font-semibold text-on-primary focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-primary";
const PANEL =
  "rounded-3xl border border-outline-variant/30 bg-surface-container p-5 sm:p-6";
function Modal({
  title,
  onClose,
  children,
}: {
  title: string;
  onClose: () => void;
  children: ReactNode;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const dialog = ref.current;
    dialog?.showModal();
    return () => dialog?.close();
  }, []);
  return (
    <dialog
      ref={ref}
      onCancel={onClose}
      onClose={onClose}
      aria-labelledby="account-modal-title"
      className="fixed inset-0 m-auto max-h-[90dvh] w-[min(92vw,36rem)] overflow-y-auto rounded-3xl border border-outline-variant bg-surface-container p-6 text-on-surface backdrop:bg-background/80"
    >
      <div className="mb-6 flex items-center justify-between gap-4">
        <h2 id="account-modal-title" className="text-headline-md font-semibold">
          {title}
        </h2>
        <button
          type="button"
          onClick={onClose}
          aria-label="Fechar"
          className="rounded-full p-2 hover:bg-surface-container-high focus-visible:outline-primary"
        >
          <span aria-hidden="true" className="material-symbols-outlined">
            close
          </span>
        </button>
      </div>
      {children}
    </dialog>
  );
}
function AddAccount({ onClose }: { onClose: () => void }) {
  const { addAccount } = useAccounts();
  const router = useRouter();
  const [manual, setManual] = useState(false);
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    const name = String(form.get("name") ?? "").trim();
    if (!name) return;
    const id = addAccount(name, String(form.get("institution") ?? ""));
    onClose();
    router.push(`/dashboard/accounts/${id}`);
  }
  return (
    <Modal title="Adicionar conta" onClose={onClose}>
      <p className="mb-4 text-body-sm text-on-surface-variant">
        Escolha como acompanhar seu patrimônio no Astra Wealth.
      </p>
      <div className="space-y-3">
        {/* TODO(Etapa 3): conectar fluxos de carteira e widget Open Finance. */}
        {["Carteira on-chain", "Banco/Corretora (Open Finance)"].map(
          (label) => (
            <button
              key={label}
              disabled
              className="flex w-full items-center justify-between rounded-2xl border border-outline-variant p-4 text-left text-on-surface-variant"
            >
              <span>{label}</span>
              <span className="ml-3 text-xs">Em breve</span>
            </button>
          ),
        )}
        <button
          type="button"
          onClick={() => setManual(true)}
          aria-expanded={manual}
          className="w-full rounded-2xl border border-primary bg-primary/10 p-4 text-left text-primary"
        >
          Lançamento manual
        </button>
      </div>
      {manual && (
        <form onSubmit={submit} className="mt-5 space-y-4">
          <label className="block text-body-sm">
            Nome da conta
            <input
              autoFocus
              name="name"
              required
              maxLength={100}
              pattern=".*\S.*"
              className={INPUT}
            />
          </label>
          <label className="block text-body-sm">
            Instituição (opcional)
            <input name="institution" maxLength={100} className={INPUT} />
          </label>
          <p className="text-body-sm text-on-surface-variant">
            Moeda base: BRL. A conta começa sem ativos. Os dados ficam apenas
            nesta sessão.
          </p>
          <button className={BUTTON}>Criar conta manual</button>
        </form>
      )}
    </Modal>
  );
}
function ContributionForm({
  accountId,
  onClose,
}: {
  accountId?: string;
  onClose: () => void;
}) {
  const { accounts, addContribution } = useAccounts();
  const [currency, setCurrency] = useState("BRL");
  const [error, setError] = useState("");
  const now = new Date();
  const localDate = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    const date = new Date(`${form.get("date")}T00:00:00`);
    const result = createContributionSchema.safeParse({
      accountId: form.get("accountId"),
      kind: form.get("kind"),
      amount: String(form.get("amount") ?? "")
        .trim()
        .replace(",", "."),
      currency,
      occurredAt: Number.isNaN(date.getTime()) ? "" : date.toISOString(),
      note: form.get("note"),
      fxRateToBrl:
        currency === "USD"
          ? String(form.get("fxRate") ?? "")
              .trim()
              .replace(",", ".")
          : null,
    });
    if (!result.success) {
      setError(result.error.issues.map((issue) => issue.message).join(" "));
      return;
    }
    try {
      addContribution(result.data);
      onClose();
    } catch (caught) {
      setError(
        caught instanceof Error
          ? caught.message
          : "Não foi possível registrar o aporte.",
      );
    }
  }
  return (
    <Modal title="Registrar aporte" onClose={onClose}>
      <form onSubmit={submit} className="space-y-4">
        <p className="text-body-sm text-on-surface-variant">
          Registro manual. Transferências entre suas carteiras não representam
          novos aportes. O registro não altera o valor atual dos ativos.
        </p>
        <label className="block text-body-sm">
          Conta
          <select
            name="accountId"
            defaultValue={accountId ?? accounts[0]?.id}
            required
            className={INPUT}
          >
            {accounts.map((account) => (
              <option key={account.id} value={account.id}>
                {account.name}
              </option>
            ))}
          </select>
        </label>
        <div className="grid grid-cols-2 gap-4">
          <label className="block text-body-sm">
            Tipo
            <select name="kind" className={INPUT}>
              <option value="DEPOSIT">Aporte</option>
              <option value="WITHDRAWAL">Retirada</option>
            </select>
          </label>
          <label className="block text-body-sm">
            Moeda
            <select
              value={currency}
              onChange={(event) => setCurrency(event.target.value)}
              className={INPUT}
            >
              <option>BRL</option>
              <option>USD</option>
            </select>
          </label>
        </div>
        <label className="block text-body-sm">
          Valor
          <input
            name="amount"
            inputMode="decimal"
            required
            placeholder="0,00"
            className={INPUT}
          />
        </label>
        {currency === "USD" && (
          <label className="block text-body-sm">
            Cotação histórica (BRL por USD)
            <input
              name="fxRate"
              inputMode="decimal"
              required
              placeholder="5,25"
              className={INPUT}
            />
          </label>
        )}
        <label className="block text-body-sm">
          Data
          <input
            name="date"
            type="date"
            required
            max={localDate}
            defaultValue={localDate}
            className={INPUT}
          />
        </label>
        <label className="block text-body-sm">
          Observação (opcional)
          <textarea name="note" maxLength={1000} rows={2} className={INPUT} />
        </label>
        {error && (
          <p role="alert" className="text-body-sm text-error">
            {error}
          </p>
        )}
        <button className={BUTTON}>Registrar nesta sessão</button>
      </form>
    </Modal>
  );
}
function AccountDetails({ account }: { account: FinancialAccountDTO }) {
  const { contributions } = useAccounts();
  const positions = [...mockHoldings, ...mockCryptoPositions].filter(
    (position) => position.accountId === account.id,
  );
  const history = contributions
    .filter((entry) => entry.accountId === account.id)
    .sort((a, b) => b.occurredAt.localeCompare(a.occurredAt));
  return (
    <>
      <ContributionsKpis summary={account} />
      <section className={PANEL}>
        <h2 className="mb-4 text-title-md font-semibold">Ativos da conta</h2>
        {positions.length ? (
          <div className="overflow-x-auto">
            <table className="w-full text-left text-body-sm">
              <thead className="text-on-surface-variant">
                <tr>
                  <th className="p-3">Ativo</th>
                  <th className="p-3">Quantidade</th>
                  <th className="p-3 text-right">Valor atual (BRL)</th>
                </tr>
              </thead>
              <tbody>
                {positions.map((position) => (
                  <tr
                    key={position.id}
                    className="border-t border-outline-variant/30"
                  >
                    <td className="p-3">
                      {position.name}
                      {position.symbol ? ` (${position.symbol})` : ""}
                    </td>
                    <td className="p-3 tabular-nums">{position.quantity}</td>
                    <td className="p-3 text-right tabular-nums">
                      {money(position.currentValue)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <p className="text-on-surface-variant">
            Esta conta ainda não possui ativos. O cadastro de posições estará
            disponível na próxima etapa.
          </p>
        )}
      </section>
      <section className={PANEL}>
        <h2 className="mb-4 text-title-md font-semibold">
          Histórico de aportes
        </h2>
        {history.length ? (
          <div className="overflow-x-auto">
            <table className="w-full text-left text-body-sm">
              <thead className="text-on-surface-variant">
                <tr>
                  {["Data", "Tipo", "Valor", "Origem", "Observação"].map(
                    (label) => (
                      <th key={label} className="p-3">
                        {label}
                      </th>
                    ),
                  )}
                </tr>
              </thead>
              <tbody>
                {history.map((entry) => (
                  <tr
                    key={entry.id}
                    className="border-t border-outline-variant/30"
                  >
                    <td className="whitespace-nowrap p-3">
                      {new Intl.DateTimeFormat("pt-BR").format(
                        new Date(entry.occurredAt),
                      )}
                    </td>
                    <td className="p-3">
                      {entry.kind === "DEPOSIT" ? "Aporte" : "Retirada"}
                    </td>
                    <td className="whitespace-nowrap p-3 tabular-nums">
                      {money(entry.amount, entry.currency)}
                      {entry.currency === "USD" && (
                        <span className="block text-xs text-on-surface-variant">
                          Cotação: {entry.fxRateToBrl} BRL/USD
                        </span>
                      )}
                    </td>
                    <td className="p-3">
                      {entry.origin === "MANUAL" ? "Manual" : "Importado"}
                    </td>
                    <td className="max-w-64 break-words p-3">
                      {entry.note || "—"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <p className="text-on-surface-variant">
            Nenhum aporte registrado. Registre o primeiro aporte desta conta.
          </p>
        )}
      </section>
    </>
  );
}
export function AccountsView({ selectedId }: { selectedId?: string }) {
  const { accounts, contributions } = useAccounts();
  const [modal, setModal] = useState<"account" | "contribution" | null>(null);
  const selected = accounts.find((account) => account.id === selectedId);
  if (selectedId && !selected)
    return (
      <section className={PANEL}>
        <h1 className="text-headline-md">Conta não encontrada nesta sessão</h1>
        <p className="my-4">
          Os lançamentos de demonstração são descartados ao recarregar a página.
        </p>
        <Link href="/dashboard/accounts" className="text-primary">
          Voltar para contas
        </Link>
      </section>
    );
  return (
    <div className="space-y-6">
      {selected && (
        <Link href="/dashboard/accounts" className="text-body-sm text-primary">
          ← Todas as contas
        </Link>
      )}
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 className="text-headline-lg font-semibold">
            {selected?.name ?? "Contas"}
          </h1>
          <p className="mt-2 text-body-sm text-on-surface-variant">
            {selected
              ? `${selected.institutionName ?? "Conta manual"} · ${syncLabel(selected)}`
              : "Seu patrimônio consolidado por conta e instituição."}
          </p>
        </div>
        <div className="flex flex-wrap gap-3">
          <button
            type="button"
            onClick={() => setModal("contribution")}
            className={BUTTON}
          >
            Registrar aporte
          </button>
          {!selected && (
            <button
              type="button"
              onClick={() => setModal("account")}
              className="rounded-full border border-outline-variant px-5 py-2.5 text-body-sm focus-visible:outline-primary"
            >
              Adicionar conta
            </button>
          )}
        </div>
      </header>
      <p
        role="note"
        className="rounded-2xl bg-primary/10 px-4 py-3 text-body-sm text-primary"
      >
        Demonstração com dados fictícios. Alterações ficam apenas nesta sessão e
        são perdidas ao recarregar.
      </p>
      {selected ? (
        <AccountDetails account={selected} />
      ) : (
        <>
          <ContributionsKpis
            summary={summarizeAccounts(accounts, contributions)}
          />
          {(Object.keys(ACCOUNT_LABELS) as AccountKind[]).map((kind) => (
            <section key={kind}>
              <h2 className="mb-3 text-title-md font-semibold">
                {ACCOUNT_LABELS[kind]}
              </h2>
              <div className="grid gap-4 xl:grid-cols-2">
                {accounts
                  .filter((account) => account.kind === kind)
                  .map((account) => (
                    <Link
                      href={`/dashboard/accounts/${account.id}`}
                      key={account.id}
                      className={`${PANEL} block transition-colors hover:border-primary/60 focus-visible:outline-2 focus-visible:outline-primary`}
                    >
                      <div className="flex items-center gap-3">
                        <span
                          aria-hidden="true"
                          className="material-symbols-outlined rounded-full bg-primary/10 p-3 text-primary"
                        >
                          {ACCOUNT_ICONS[kind]}
                        </span>
                        <div className="min-w-0">
                          <h3 className="truncate font-semibold">
                            {account.name}
                          </h3>
                          <p className="text-body-sm text-on-surface-variant">
                            {account.institutionName ?? "Conta manual"}
                          </p>
                        </div>
                      </div>
                      <p className="mt-5 text-body-sm text-on-surface-variant">
                        Valor atual
                      </p>
                      <p className="mt-1 text-2xl font-semibold tabular-nums">
                        {money(account.currentValue)}
                      </p>
                      <dl className="mt-4 space-y-2 text-body-sm">
                        <div className="flex flex-wrap justify-between gap-2">
                          <dt>Total aportado</dt>
                          <dd>{money(account.totalContributed)}</dd>
                        </div>
                        <div className="flex flex-wrap justify-between gap-2">
                          <dt>Resultado</dt>
                          <dd
                            className={
                              Number(account.result) > 0
                                ? "text-secondary"
                                : Number(account.result) < 0
                                  ? "text-error"
                                  : "text-on-surface-variant"
                            }
                          >
                            {money(account.result)} ·{" "}
                            {percent(account.resultPct)}
                          </dd>
                        </div>
                      </dl>
                      <p
                        className={`mt-4 text-xs ${account.syncStatus === "NEEDS_REAUTH" ? "text-error" : "text-on-surface-variant"}`}
                      >
                        {syncLabel(account)}
                      </p>
                    </Link>
                  ))}
              </div>
            </section>
          ))}
        </>
      )}
      {modal === "account" && <AddAccount onClose={() => setModal(null)} />}
      {modal === "contribution" && (
        <ContributionForm
          {...(selected ? { accountId: selected.id } : {})}
          onClose={() => setModal(null)}
        />
      )}
    </div>
  );
}
