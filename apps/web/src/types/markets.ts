export type AssetClass = "crypto" | "stocks" | "etfs" | "indices" | "forex" | "commodities";

export const ASSET_CLASS_LABELS: Record<AssetClass, string> = {
  crypto: "Cripto", stocks: "Ações", etfs: "ETFs", indices: "Índices", forex: "Câmbio", commodities: "Commodities",
};

export interface MarketInstrument {
  id: string;
  symbol: string;
  name: string;
  assetClass: AssetClass;
  venue: string;
  currency: "USD" | "BRL";
  price: number;
  change: number;
  unit?: string;
}

export function formatMarketPrice(asset: MarketInstrument): string {
  if (asset.assetClass === "indices") return `${new Intl.NumberFormat("pt-BR", { maximumFractionDigits: 2 }).format(asset.price)} pts`;
  return new Intl.NumberFormat("pt-BR", { style: "currency", currency: asset.currency, maximumFractionDigits: asset.price < 1 ? 4 : 2 }).format(asset.price);
}
