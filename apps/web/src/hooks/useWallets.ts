"use client";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { usePrivateFetch } from "@/lib/private-session-context";
import type { WalletDto, WalletChain } from "../../../../packages/shared/src/generated/rust/WalletDto";
import type { WalletAssetsResponse } from "../../../../packages/shared/src/generated/rust/WalletAssetsResponse";
export type { WalletDto, WalletAssetsResponse };
export function useWallets() {
  const privateFetch = usePrivateFetch();
  return useQuery({ queryKey: ["private", "wallets"], queryFn: ({ signal }) => privateFetch<WalletDto[]>("wallets", { signal }), retry: false });
}
export function useWalletAssets(id?: string) {
  const privateFetch = usePrivateFetch();
  return useQuery({ queryKey: ["private", "wallet-assets", id], queryFn: ({ signal }) => privateFetch<WalletAssetsResponse>(`wallets/${encodeURIComponent(id!)}/assets`, { signal }), enabled: !!id, retry: false, staleTime: 60000 });
}
export function useWalletMutations() {
  const privateFetch = usePrivateFetch();
  const client = useQueryClient();
  const add = useMutation({ mutationFn: (input: { nickname: string; address: string; chain: WalletChain }) => privateFetch<WalletDto>("wallets", { method: "POST", body: JSON.stringify(input) }), onSuccess: () => client.invalidateQueries({ queryKey: ["private", "wallets"] }) });
  const sync = useMutation({ mutationFn: (id: string) => privateFetch<WalletAssetsResponse>(`wallets/${encodeURIComponent(id)}/sync`, { method: "POST", body: "{}" }), onSuccess: (data) => { client.setQueryData(["private", "wallet-assets", data.walletId], data); } });
  const remove = useMutation({ mutationFn: (id: string) => privateFetch<void>(`wallets/${encodeURIComponent(id)}`, { method: "DELETE" }), onSuccess: (_, id) => { client.removeQueries({ queryKey: ["private", "wallet-assets", id] }); return client.invalidateQueries({ queryKey: ["private", "wallets"] }); } });
  return { add, sync, remove };
}
