import type { FastifyInstance } from "fastify";
import type { WalletBalance, WalletSummary } from "@astra-wealth/shared";
import { AlchemyClient } from "../../clients/alchemy.js";
import { MoralisClient } from "../../clients/moralis.js";

const CACHE_TTL_SECONDS = 30; // menor que o market (60s) — saldo on-chain muda mais devagar, mas queremos refletir rápido

export class WalletService {
  private readonly alchemy: AlchemyClient;
  private readonly moralis: MoralisClient;

  constructor(private readonly fastify: FastifyInstance) {
    this.alchemy = new AlchemyClient(process.env.ALCHEMY_API_KEY ?? "");
    this.moralis = new MoralisClient(process.env.MORALIS_API_KEY ?? "");
  }

  async getSummary(address: string): Promise<WalletSummary> {
    const cacheKey = `wallet:${address.toLowerCase()}`;
    const cached = await this.fastify.redis.get(cacheKey);

    if (cached) {
      return JSON.parse(cached) as WalletSummary;
    }

    const [alchemyResult, moralisResult] = await Promise.allSettled([
      this.alchemy.getNativeBalance(address),
      this.moralis.getNativeBalance(address),
    ]);

    const alchemy: WalletBalance | null =
      alchemyResult.status === "fulfilled"
        ? { address, nativeBalance: alchemyResult.value, source: "alchemy" }
        : null;

    const moralis: WalletBalance | null =
      moralisResult.status === "fulfilled"
        ? { address, nativeBalance: moralisResult.value, source: "moralis" }
        : null;

    const summary: WalletSummary = {
      address,
      alchemy,
      moralis,
      fetchedAt: new Date().toISOString(),
    };

    await this.fastify.redis.set(
      cacheKey,
      JSON.stringify(summary),
      "EX",
      CACHE_TTL_SECONDS
    );

    return summary;
  }
}