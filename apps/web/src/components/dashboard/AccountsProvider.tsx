"use client";
import { createContext, useContext, type ReactNode } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { calculateContributionsSummary, createContributionSchema, sumDecimalValues,
  type FinancialAccountDTO, type ContributionDTO, type HoldingDTO, type CreateContributionInput,
} from "@astra-wealth/shared/accounts";
import { privateFetch } from "@/lib/private-api";
import type { AccountRecord, AccountDetail } from "@/types/accounts-api";

interface AccountsState {
  accounts: FinancialAccountDTO[];
  contributions: ContributionDTO[];
  holdings: HoldingDTO[];
  unavailableValues: string[];
  loading: boolean;
  error: Error | null;
  refresh: () => void;
  addAccount: (name: string, institutionName: string) => Promise<string>;
  addContribution: (input: CreateContributionInput) => Promise<void>;
}
const Context = createContext<AccountsState | null>(null);
export function AccountsProvider({ children }: { children: ReactNode }) {
  const client = useQueryClient();
  const query = useQuery({ queryKey: ["private", "accounts"], retry: false,
    queryFn: async ({ signal }) => {
      const list = await privateFetch<AccountRecord[]>("accounts", { signal });
      const details: AccountDetail[] = [];
      for (let offset = 0; offset < list.length; offset += 5) {
        details.push(...await Promise.all(list.slice(offset, offset + 5).map((account) =>
          privateFetch<AccountDetail>(`accounts/${encodeURIComponent(account.id)}`, { signal }))));
      }
      return details;
    },
  });
  const records = query.data ?? [];
  const unavailableValues = records.filter((a) => a.kind === "WALLET" || a.baseCurrency !== "BRL").map((a) => a.id);
  const accounts: FinancialAccountDTO[] = records.map((account) => ({
    ...account, syncStatus: account.kind === "WALLET" ? "ACTIVE" : "MANUAL",
    ...calculateContributionsSummary(account.contributions,
      unavailableValues.includes(account.id) ? "0" : sumDecimalValues(account.holdings.map((h) => h.currentValue))),
  }));
  const create = useMutation({ mutationFn: (input: { name: string; institutionName: string | null }) =>
    privateFetch<AccountRecord>("accounts", { method: "POST", body: JSON.stringify({ ...input, kind: "MANUAL", baseCurrency: "BRL" }) }),
    onSuccess: (account) => { client.setQueryData<AccountDetail[]>(["private", "accounts"], (old = []) => [{ ...account, holdings: [], contributions: [] }, ...old]); },
  });
  const contribute = useMutation({ mutationFn: (input: CreateContributionInput) => {
    const { accountId, ...body } = createContributionSchema.parse(input);
    return privateFetch<ContributionDTO>(`accounts/${encodeURIComponent(accountId)}/contributions`, { method: "POST", body: JSON.stringify(body) });
  }, onSuccess: (entry) => { client.setQueryData<AccountDetail[]>(["private", "accounts"], (old = []) => old.map((a) => a.id === entry.accountId ? { ...a, contributions: [entry, ...a.contributions] } : a)); } });
  return <Context.Provider value={{ accounts, contributions: records.flatMap((a) => a.contributions),
    holdings: records.flatMap((a) => a.holdings), unavailableValues, loading: query.isPending, error: query.error,
    refresh: () => { void query.refetch(); },
    addAccount: async (name, institutionName) => (await create.mutateAsync({ name: name.trim(), institutionName: institutionName.trim() || null })).id,
    addContribution: async (input) => { await contribute.mutateAsync(input); },
  }}>{children}</Context.Provider>;
}
export function useAccounts() {
  const context = useContext(Context);
  if (!context) throw new Error("AccountsProvider não encontrado.");
  return context;
}
