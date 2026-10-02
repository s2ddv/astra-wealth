"use client";

import { createContext, useContext, useState, type ReactNode } from "react";
import {
  calculateContributionsSummary,
  createContributionSchema,
  type FinancialAccountDTO,
  type ContributionDTO,
  type CreateContributionInput,
} from "@astra-wealth/shared/accounts";
import { mockAccounts, mockContributions } from "@/mocks/accounts";

interface AccountsState {
  accounts: FinancialAccountDTO[];
  contributions: ContributionDTO[];
  addAccount: (name: string, institutionName: string) => string;
  addContribution: (input: CreateContributionInput) => void;
}
const Context = createContext<AccountsState | null>(null);
export function AccountsProvider({ children }: { children: ReactNode }) {
  // TODO(Etapa 2): substituir estado em memória por consultas e mutações da API /v1/accounts.
  const [baseAccounts, setAccounts] = useState(mockAccounts);
  const [contributions, setContributions] = useState(mockContributions);
  const accounts = baseAccounts.map((account) => ({
    ...account,
    ...calculateContributionsSummary(
      contributions.filter((item) => item.accountId === account.id),
      account.currentValue,
    ),
  }));
  function addAccount(name: string, institutionName: string) {
    const id = crypto.randomUUID();
    if (!name.trim()) throw new Error("Informe o nome da conta.");
    setAccounts((current) => [
      ...current,
      {
        id,
        kind: "MANUAL",
        name: name.trim(),
        institutionName: institutionName.trim() || null,
        baseCurrency: "BRL",
        syncStatus: "MANUAL",
        ...calculateContributionsSummary([], "0"),
      },
    ]);
    return id;
  }
  function addContribution(input: CreateContributionInput) {
    const parsed = createContributionSchema.parse(input);
    if (!baseAccounts.some((account) => account.id === parsed.accountId))
      throw new Error("Conta não encontrada.");
    setContributions((current) => [
      ...current,
      {
        ...parsed,
        id: crypto.randomUUID(),
        note: parsed.note || null,
        fxRateToBrl: parsed.fxRateToBrl ?? null,
        origin: "MANUAL",
        externalId: null,
        createdAt: new Date().toISOString(),
      },
    ]);
  }
  return (
    <Context.Provider
      value={{ accounts, contributions, addAccount, addContribution }}
    >
      {children}
    </Context.Provider>
  );
}
export function useAccounts() {
  const context = useContext(Context);
  if (!context) throw new Error("AccountsProvider não encontrado.");
  return context;
}
