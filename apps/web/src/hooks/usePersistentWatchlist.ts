"use client";
import { useRef } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { PrivateApiError, privateFetch } from "@/lib/private-api";
interface WatchlistItem { id: string; coinId: string }
interface Watchlist { id: string; name: string; items: WatchlistItem[] }
const key = ["private", "watchlists"];
const displayId = (id: string) => id.includes(":") ? id : `crypto:${id}`;
const storedId = (id: string) => id.startsWith("crypto:") ? id.slice(7) : id;
export function usePersistentWatchlist() {
  const client = useQueryClient();
  const busy = useRef(false);
  const query = useQuery({ queryKey: key, queryFn: ({ signal }) => privateFetch<Watchlist[]>("watchlists", { signal }), retry: false });
  const list = query.data?.find((entry) => entry.name === "Markets");
  const mutation = useMutation({ mutationFn: async (id: string) => {
    if (!query.data) throw new Error("Carregue sua watchlist antes de editar.");
    let target = list;
    if (!target) {
      try { target = await privateFetch<Watchlist>("watchlists", { method: "POST", body: JSON.stringify({ name: "Markets" }) }); }
      catch (error) {
        if (!(error instanceof PrivateApiError) || error.status !== 409) throw error;
        target = (await privateFetch<Watchlist[]>("watchlists")).find((entry) => entry.name === "Markets");
        if (!target) throw error;
      }
    }
    const existing = target.items.find((item) => displayId(item.coinId) === id);
    if (!existing && target.items.length >= 100) throw new Error("Sua watchlist pode acompanhar até 100 ativos.");
    const prefix = `watchlists/${encodeURIComponent(target.id)}/items`;
    if (existing) {
      await privateFetch<void>(`${prefix}/${encodeURIComponent(existing.coinId)}`, { method: "DELETE" });
      return { ...target, items: target.items.filter((item) => item.id !== existing.id) };
    }
    const item = await privateFetch<WatchlistItem>(prefix, { method: "POST", body: JSON.stringify({ coinId: storedId(id) }) });
    return { ...target, items: [...target.items, item] };
  }, onSuccess: (updated) => {
    client.setQueryData<Watchlist[]>(key, (old = []) => [...old.filter((item) => item.id !== updated.id), updated]);
  } });
  return { ids: list?.items.map((item) => displayId(item.coinId)) ?? [], ready: !query.isPending,
    error: query.error ?? mutation.error, refresh: () => { void query.refetch(); },
    toggle: async (id: string) => {
      if (busy.current) return false;
      busy.current = true;
      try { await mutation.mutateAsync(id); return true; } finally { busy.current = false; }
    },
  };
}
