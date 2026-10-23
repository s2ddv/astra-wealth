"use client";

import { useEffect, useState } from "react";
import { apiFetch } from "../lib/api";

import type { MarketCoinDto as SpotCoin } from "../../../../packages/shared/src/generated/rust/MarketDto";

export function useSpot(limit = 10) {
  const [data, setData] = useState<{ data: SpotCoin[] } | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isError, setIsError] = useState(false);

  useEffect(() => {
    setIsLoading(true);
    setIsError(false);
    const controller = new AbortController();
    apiFetch<SpotCoin[]>(`/v1/market/spot?limit=${limit}`, { signal: controller.signal })
      .then((coins) => { if (!controller.signal.aborted) setData({ data: coins }); })
      .catch(() => { if (!controller.signal.aborted) setIsError(true); })
      .finally(() => { if (!controller.signal.aborted) setIsLoading(false); });
    return () => controller.abort();
  }, [limit]);

  return { data, isLoading, isError };
}