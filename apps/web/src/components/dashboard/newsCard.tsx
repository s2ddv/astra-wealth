"use client";
import { useState } from "react";
import type { NewsArticleDto } from "@zora-wealth/shared";

const relative = new Intl.RelativeTimeFormat("pt-BR", { numeric: "auto" });
function relativeTime(date: string, now: number) {
  const seconds = Math.round((Date.parse(date) - now) / 1000);
  if (!Number.isFinite(seconds)) return "Data indisponível";
  for (const [unit, size] of [["day", 86400], ["hour", 3600], ["minute", 60]] as const) {
    if (Math.abs(seconds) >= size) return relative.format(Math.trunc(seconds / size), unit);
  }
  return relative.format(seconds, "second");
}

export function NewsCard({ item, referenceTime }: { item: NewsArticleDto; referenceTime: number }) {
  const [failedImage, setFailedImage] = useState<string | null>(null);
  return (
    <article className="h-full overflow-hidden rounded-3xl border border-outline-variant/30 bg-surface-container">
      <div className="flex aspect-[16/7] items-center justify-center overflow-hidden bg-surface-container-high text-on-surface-variant">
        {item.imageUrl && failedImage !== item.imageUrl ? (
          // Publisher image hosts vary. The request carries no referrer and has a local fallback.
          // eslint-disable-next-line @next/next/no-img-element
          <img src={item.imageUrl} alt="" loading="lazy" referrerPolicy="no-referrer" className="h-full w-full object-cover" onError={() => setFailedImage(item.imageUrl)} />
        ) : <span aria-hidden="true" className="material-symbols-outlined text-5xl">newspaper</span>}
      </div>
      <div className="p-6">
        <div className="mb-4 flex flex-wrap items-center gap-3 text-body-sm text-on-surface-variant">
          <span>{item.sourceName}</span>
          <time dateTime={item.publishedAt} title={new Date(item.publishedAt).toLocaleString("pt-BR")}>{relativeTime(item.publishedAt, referenceTime)}</time>
        </div>
        <h2 className="text-title-md font-semibold">
          <a href={item.url} target="_blank" rel="noopener noreferrer" className="rounded hover:text-primary focus-visible:outline-2 focus-visible:outline-primary">{item.title}<span className="sr-only"> (abre em nova aba)</span></a>
        </h2>
        {item.summary && <p className="mt-3 text-body-base text-on-surface-variant">{item.summary}</p>}
        <span className="mt-5 inline-flex rounded-full bg-primary/10 px-3 py-1 text-body-sm text-primary">{item.category === "crypto" ? "Cripto" : "Economia"}</span>
      </div>
    </article>
  );
}
