import type { NewsItemDto } from "@/types/dashboard";

// MOCK: substituir na Fase 9 pelo provedor de notícias e seus URLs reais.
// Notícias fictícias sem links externos; relógio fixo para uma demonstração reproduzível.
export const mockNewsReferenceTime = "2026-09-24T18:00:00Z";
export const mockNews: NewsItemDto[] = [
  { id: "news-1", title: "Bitcoin e Ethereum lideram o interesse dos investidores nesta semana", source: "Astra Research · Demo", category: "crypto", publishedAt: "2026-09-24T17:45:00Z", url: "" },
  { id: "news-2", title: "Mercados acompanham a agenda de juros e os próximos dados de inflação", source: "Macro Brief · Demo", category: "macro", publishedAt: "2026-09-24T17:00:00Z", url: "" },
  { id: "news-3", title: "Redes de segunda camada ampliam as possibilidades para aplicações on-chain", source: "On-chain Daily · Demo", category: "crypto", publishedAt: "2026-09-24T15:00:00Z", url: "" },
  { id: "news-4", title: "Dólar e títulos públicos estão no radar da diversificação global", source: "Macro Brief · Demo", category: "macro", publishedAt: "2026-09-24T12:00:00Z", url: "" },
  { id: "news-5", title: "Ecossistema Solana apresenta novas ferramentas para desenvolvedores", source: "On-chain Daily · Demo", category: "crypto", publishedAt: "2026-09-23T16:00:00Z", url: "" },
  { id: "news-6", title: "Atividade econômica global orienta as expectativas para o próximo trimestre", source: "Astra Research · Demo", category: "macro", publishedAt: "2026-09-22T18:00:00Z", url: "" },
];
