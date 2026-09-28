"use client";
import { useQuery } from "@tanstack/react-query";
import type { MarketCoinDto, MarketPageDto } from "../../../../packages/shared/src/generated/rust/MarketDto";
export type { MarketCoinDto };

async function marketFetch<T>(path: string, signal: AbortSignal): Promise<T> {
  const response = await fetch(`/api/market/${path}`, { signal });
  if (!response.ok) throw new Error(response.status === 404 ? "Ativo não encontrado." : "Dados indisponíveis. Tente novamente em instantes.");
  return response.json() as Promise<T>;
}
const polling = { staleTime: 60_000, refetchInterval: 60_000, retry: false as const };
export function useMarkets(page: number) {
  return useQuery({ queryKey: ["markets", page], queryFn: ({ signal }) => marketFetch<MarketPageDto>(`coins?page=${page}&perPage=50`, signal), ...polling });
}
export function useWatchlistMarkets(ids: string[], ready: boolean) {
  const canonical = [...ids].sort().join(",");
  return useQuery({ queryKey: ["market-watchlist", canonical], queryFn: ({ signal }) => marketFetch<MarketPageDto>(`coins?perPage=100&ids=${encodeURIComponent(canonical)}`, signal), enabled: ready && ids.length > 0, ...polling });
}
export function useTrendingMarkets() {
  return useQuery({ queryKey: ["market-trending"], queryFn: ({ signal }) => marketFetch<MarketCoinDto[]>("trending", signal), staleTime: 300_000, refetchInterval: 300_000, retry: false });
}
export function useMarketCoin(id: string) {
  return useQuery({ queryKey: ["market-coin", id], queryFn: ({ signal }) => marketFetch<MarketCoinDto>(`coins/${encodeURIComponent(id)}`, signal), ...polling });
}
