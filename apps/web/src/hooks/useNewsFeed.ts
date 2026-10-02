import { useInfiniteQuery } from "@tanstack/react-query";
import type { NewsArticleDto, NewsPageDto } from "@astra-wealth/shared";
export type NewsCategoryFilter = NewsArticleDto["category"] | "all";
export type NewsLanguageFilter = NewsArticleDto["language"] | "all";

export function useNewsFeed(category: NewsCategoryFilter, lang: NewsLanguageFilter) {
  return useInfiniteQuery({
    queryKey: ["news-feed", category, lang],
    initialPageParam: null as string | null,
    queryFn: async ({ pageParam, signal }): Promise<NewsPageDto> => {
      const params = new URLSearchParams({ category, lang, limit: "20" });
      if (pageParam) params.set("cursor", pageParam);
      const response = await fetch(`/api/news?${params}`, { signal });
      if (!response.ok) throw new Error(response.status === 401 ? "Entre na sua conta para ler as notícias." : "Não foi possível carregar as notícias.");
      return response.json();
    },
    getNextPageParam: (page) => page.nextCursor,
    staleTime: 60_000,
    refetchInterval: 300_000,
    retry: false,
  });
}
