"use client";

import { useState } from "react";
import { Search } from "lucide-react";
import { useSpot } from "../../hooks/useSpot";

function formatCompact(value: number | null) {
  if (value === null) return "—";
  return new Intl.NumberFormat("en-US", {
    notation: "compact",
    maximumFractionDigits: 2,
  }).format(value);
}

function formatPrice(value: number | null) {
  if (value === null) return "—";
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: value < 1 ? 4 : 2,
    maximumFractionDigits: value < 1 ? 4 : 2,
  }).format(value);
}

export function CoinsCard() {
  const { data, isLoading, isError } = useSpot();
  const [query, setQuery] = useState("");

  const coins = data?.data.filter(
    (coin) =>
      coin.symbol.toLowerCase().includes(query.toLowerCase()) ||
      coin.name.toLowerCase().includes(query.toLowerCase())
  );

  const maxMarketCap = coins?.length
    ? Math.max(1, ...coins.map((c) => c.marketCap ?? 0))
    : 1;

  return (
    <div
      className="liquid-glass rounded-xl p-6 h-full overflow-y-auto no-scrollbar"
      data-lenis-prevent
    >
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-base font-semibold text-neutral-100">Spot</h2>

        <div className="relative w-48">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-neutral-500" />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search..."
            className="w-full bg-neutral-800/60 text-neutral-200 placeholder-neutral-500 text-xs rounded-lg pl-8 pr-3 py-1.5 outline-none focus:ring-1 focus:ring-neutral-600"
          />
        </div>
      </div>

      {isLoading && <p className="text-neutral-400 text-xs">Loading...</p>}

      {isError && (
        <p className="text-red-400 text-xs">Failed to load market data.</p>
      )}

      {coins && (
        <table className="w-full text-xs border-separate border-spacing-0">
          <thead>
            <tr className="text-left text-neutral-500 text-[10px] uppercase tracking-wide">
              <th className="font-medium pb-2 border-b border-neutral-700/60">
                Asset
              </th>
              <th className="font-medium pb-2 border-b border-neutral-700/60">
                Price
              </th>
              <th className="font-medium pb-2 border-b border-neutral-700/60">
                Market Cap
              </th>
            </tr>
          </thead>
          <tbody>
            {coins.map((coin) => {
              const isPositive = coin.priceChangePercentage24h !== null && coin.priceChangePercentage24h >= 0;
              const barWidth = Math.max(
                4,
                ((coin.marketCap ?? 0) / maxMarketCap) * 100
              );

              return (
                <tr
                  key={coin.id}
                  className="border-b border-neutral-800/40 last:border-0"
                >
                  <td className="py-2 pr-3">
                    <div className="flex items-center gap-1.5">
                      {coin.image && <img
                        src={coin.image}
                        alt={coin.name}
                        className="w-5 h-5 rounded-full"
                      />}
                      <span className="text-neutral-100 text-xs font-medium">
                        {coin.symbol.toUpperCase()}
                      </span>
                    </div>
                  </td>

                  <td className="py-2 pr-3">
                    <div className="flex items-center gap-1.5">
                      <span className="text-neutral-100 font-mono text-xs">
                        {formatPrice(coin.currentPrice)}
                      </span>
                      <span
                        className={`text-[10px] font-mono ${
                          isPositive ? "text-emerald-400" : "text-red-400"
                        }`}
                      >
                        {coin.priceChangePercentage24h === null ? "—" : `${isPositive ? "+" : "-"}${Math.abs(coin.priceChangePercentage24h).toFixed(2)}%`}
                      </span>
                    </div>
                  </td>

                  <td className="py-2">
                    <div className="flex flex-col gap-1">
                      <span className="text-neutral-100 font-mono text-[10px]">
                        {formatCompact(coin.marketCap)}
                      </span>
                      <div className="h-1 w-20 bg-neutral-800/60 rounded-full overflow-hidden">
                        <div
                          className={`h-full rounded-full ${
                            isPositive ? "bg-emerald-500" : "bg-red-500"
                          }`}
                          style={{ width: `${barWidth}%` }}
                        />
                      </div>
                    </div>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
}