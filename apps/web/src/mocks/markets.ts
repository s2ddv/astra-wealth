import type { MarketInstrument } from "@/types/markets";

// Illustrative quotes only. No live market data or trading integration.
export const mockMarketAssets: MarketInstrument[] = [
  { id: "crypto:btc", symbol: "BTC", name: "Bitcoin", assetClass: "crypto", venue: "Cripto · USD", currency: "USD", price: 67432.18, change: 2.31 },
  { id: "crypto:eth", symbol: "ETH", name: "Ethereum", assetClass: "crypto", venue: "Cripto · USD", currency: "USD", price: 3482.11, change: 1.84 },
  { id: "crypto:sol", symbol: "SOL", name: "Solana", assetClass: "crypto", venue: "Cripto · USD", currency: "USD", price: 178.4, change: -0.92 },
  { id: "crypto:link", symbol: "LINK", name: "Chainlink", assetClass: "crypto", venue: "Cripto · USD", currency: "USD", price: 19.29, change: -2.41 },
  { id: "crypto:usdt", symbol: "USDT", name: "Tether", assetClass: "crypto", venue: "Cripto · USD", currency: "USD", price: 1, change: 0 },
  { id: "nasdaq:aapl", symbol: "AAPL", name: "Apple", assetClass: "stocks", venue: "NASDAQ", currency: "USD", price: 213.07, change: 1.24 },
  { id: "nasdaq:nvda", symbol: "NVDA", name: "NVIDIA", assetClass: "stocks", venue: "NASDAQ", currency: "USD", price: 126.57, change: 3.52 },
  { id: "nasdaq:msft", symbol: "MSFT", name: "Microsoft", assetClass: "stocks", venue: "NASDAQ", currency: "USD", price: 428.76, change: -0.45 },
  { id: "nasdaq:tsla", symbol: "TSLA", name: "Tesla", assetClass: "stocks", venue: "NASDAQ", currency: "USD", price: 248.5, change: -1.76 },
  { id: "b3:petr4", symbol: "PETR4", name: "Petrobras PN", assetClass: "stocks", venue: "B3", currency: "BRL", price: 38.72, change: 0.83 },
  { id: "b3:vale3", symbol: "VALE3", name: "Vale ON", assetClass: "stocks", venue: "B3", currency: "BRL", price: 61.42, change: -1.12 },
  { id: "arca:spy", symbol: "SPY", name: "SPDR S&P 500 ETF", assetClass: "etfs", venue: "NYSE Arca", currency: "USD", price: 548.49, change: 0.76 },
  { id: "nasdaq:qqq", symbol: "QQQ", name: "Invesco QQQ Trust", assetClass: "etfs", venue: "NASDAQ", currency: "USD", price: 479.11, change: 1.12 },
  { id: "b3:bova11", symbol: "BOVA11", name: "iShares Ibovespa", assetClass: "etfs", venue: "B3", currency: "BRL", price: 124.38, change: -0.32 },
  { id: "index:spx", symbol: "SPX", name: "S&P 500", assetClass: "indices", venue: "Estados Unidos", currency: "USD", price: 5482.87, change: 0.77 },
  { id: "index:ibov", symbol: "IBOV", name: "Ibovespa", assetClass: "indices", venue: "Brasil", currency: "BRL", price: 128320.5, change: -0.34 },
  { id: "forex:usdbrl", symbol: "USD/BRL", name: "Dólar / Real", assetClass: "forex", venue: "Forex", currency: "BRL", price: 5.43, change: 0.28, unit: "por USD" },
  { id: "forex:eurusd", symbol: "EUR/USD", name: "Euro / Dólar", assetClass: "forex", venue: "Forex", currency: "USD", price: 1.09, change: -0.15, unit: "por EUR" },
  { id: "commodity:gold", symbol: "XAU/USD", name: "Ouro", assetClass: "commodities", venue: "Spot", currency: "USD", price: 2368.8, change: 1.07, unit: "por onça troy" },
  { id: "commodity:brent", symbol: "BRENT", name: "Petróleo Brent", assetClass: "commodities", venue: "Referência Brent", currency: "USD", price: 85.24, change: -0.64, unit: "por barril" },
];

export const defaultWatchlist = ["crypto:btc", "crypto:eth", "nasdaq:nvda", "b3:petr4"];
export const marketHighlights = ["crypto:btc", "index:spx", "index:ibov", "forex:usdbrl"];
