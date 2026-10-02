import { useQuery } from "@tanstack/react-query";
import type { WalletSummary } from "@astra-wealth/shared";
import { apiFetch } from "../lib/api";

const ADDRESS_REGEX = /^0x[a-fA-F0-9]{40}$/;

export function useWalletSummary(address: string | null) {
  return useQuery({
    queryKey: ["wallet", address],
    queryFn: () => apiFetch<WalletSummary>(`/v1/wallet/${address}`),
    enabled: !!address && ADDRESS_REGEX.test(address),
    refetchInterval: 30_000,
  });
}