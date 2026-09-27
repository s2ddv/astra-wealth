"use client";

export function WatchlistButton({ name, selected, onToggle }: { name: string; selected: boolean; onToggle: () => void }) {
  return (
    <button type="button" aria-label={`${selected ? "Remover" : "Adicionar"} ${name} ${selected ? "da" : "à"} watchlist`} aria-pressed={selected} onClick={onToggle}
      className={`flex h-10 w-10 shrink-0 items-center justify-center rounded-full transition-colors hover:bg-primary/10 focus-visible:outline-2 focus-visible:outline-primary ${selected ? "text-primary" : "text-on-surface-variant"}`}>
      <svg aria-hidden="true" width="20" height="20" viewBox="0 0 24 24" fill={selected ? "currentColor" : "none"} stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round"><path d="m12 3 2.78 5.63L21 9.54l-4.5 4.39 1.06 6.2L12 17.2l-5.56 2.93 1.06-6.2L3 9.54l6.22-.91Z" /></svg>
    </button>
  );
}
