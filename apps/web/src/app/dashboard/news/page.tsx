"use client";
import { useEffect, useState } from "react";
import { NewsCard } from "@/components/dashboard/newsCard";
import { PageHeader } from "@/components/dashboard/PageHeader";
import { useNewsFeed, type NewsCategoryFilter, type NewsLanguageFilter } from "@/hooks/useNewsFeed";

const categories = [{ value: "all", label: "Todas" }, { value: "crypto", label: "Cripto" }, { value: "macro", label: "Economia" }] as const;
const languages = [{ value: "pt", label: "PT" }, { value: "en", label: "EN" }, { value: "all", label: "Todos" }] as const;
const button = "rounded-full border px-5 py-2 text-body-sm font-medium transition-colors focus-visible:outline-2 focus-visible:outline-primary disabled:opacity-50";
function chip(active: boolean) { return `${button} ${active ? "border-primary bg-primary text-on-primary" : "border-outline-variant bg-surface-container text-on-surface-variant hover:bg-surface-container-high"}`; }

export default function NewsPage() {
  const [category, setCategory] = useState<NewsCategoryFilter>("all");
  const [language, setLanguage] = useState<NewsLanguageFilter>("all");
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => { const timer = setInterval(() => setNow(Date.now()), 60_000); return () => clearInterval(timer); }, []);
  const feed = useNewsFeed(category, language);
  // Live snapshots can shift between pages; never render the same article twice.
  const items = [...new Map(feed.data?.pages.flatMap((page) => page.data).map((item) => [item.id, item])).values()];
  return (
    <div className="flex flex-col gap-8">
      <PageHeader eyebrow="Insights" title="Notícias" description="Cripto e economia, direto das fontes. Algumas notícias podem chegar com atraso de até 12 horas." />
      <div className="flex flex-col gap-4">
        <div role="group" aria-label="Filtrar notícias por categoria" className="flex flex-wrap gap-2">
          {categories.map((filter) => <button key={filter.value} aria-pressed={category === filter.value} onClick={() => setCategory(filter.value)} className={chip(category === filter.value)}>{filter.label}</button>)}
        </div>
        <div role="group" aria-label="Filtrar notícias por idioma" className="flex flex-wrap items-center gap-2">
          <span className="mr-2 text-body-sm text-on-surface-variant">Idioma</span>
          {languages.map((filter) => <button key={filter.value} aria-pressed={language === filter.value} onClick={() => setLanguage(filter.value)} className={chip(language === filter.value)}>{filter.label}</button>)}
        </div>
      </div>
      <section aria-label="Feed de notícias" aria-busy={feed.isFetching}>
        {feed.isPending && <div role="status"><span className="sr-only">Carregando notícias</span><div className="grid gap-4 xl:grid-cols-2">{Array.from({ length: 6 }, (_, index) => <div key={index} className="h-80 animate-pulse rounded-3xl bg-surface-container-high motion-reduce:animate-none" />)}</div></div>}
        {feed.isError && <div role="alert" className="mb-6 rounded-3xl border border-outline-variant p-6"><p className="mb-4 text-on-surface-variant">{feed.error.message}</p><button disabled={feed.isFetching} className={chip(false)} onClick={() => void (feed.isFetchNextPageError ? feed.fetchNextPage() : feed.refetch())}>Tentar novamente</button></div>}
        {feed.data?.pages.some((page) => page.stale) && <p role="status" className="mb-4 text-body-sm text-on-surface-variant">Exibindo a última atualização disponível. As fontes estão temporariamente indisponíveis.</p>}
        {!feed.isPending && !feed.isError && items.length === 0 && <p role="status" className="rounded-3xl bg-surface-container p-8 text-on-surface-variant">Nenhuma notícia encontrada para estes filtros.</p>}
        {items.length > 0 && <ul className="grid gap-4 xl:grid-cols-2">{items.map((item) => <li key={item.id}><NewsCard item={item} referenceTime={now} /></li>)}</ul>}
        {feed.hasNextPage && <div className="mt-6 text-center"><button className={chip(false)} disabled={feed.isFetching} onClick={() => void feed.fetchNextPage()}>{feed.isFetchingNextPage ? "Carregando…" : "Carregar mais"}</button></div>}
      </section>
    </div>
  );
}
