import type { FastifyInstance } from "fastify";
import type { CreateWalletInput } from "@astra-wealth/shared";
import { WalletRepository } from "../../repositories/wallet.repository.js";
import { WalletAssetRepository } from "../../repositories/wallet-asset.repository.js";
import { WalletService } from "../wallet/wallet.service.js";
import { toWalletDto } from "../../lib/mappers.js";

export class MeWalletsService {
  private readonly walletRepo: WalletRepository;
  private readonly walletAssetRepo: WalletAssetRepository;
  private readonly onChainService: WalletService;

  constructor(private readonly fastify: FastifyInstance) {
    this.walletRepo = new WalletRepository(fastify.prisma);
    this.walletAssetRepo = new WalletAssetRepository(fastify.prisma);
    this.onChainService = new WalletService(fastify);
  }

  async list(userId: string) {
    const wallets = await this.walletRepo.findByUserId(userId);
    return wallets.map(toWalletDto);
  }

  async create(userId: string, input: CreateWalletInput) {
    const wallet = await this.walletRepo.create({
      userId,
      address: input.address,
      chain: input.chain ?? "ETHEREUM",
      nickname: input.nickname ?? null,
    });
    await this.syncAssets(wallet.id, wallet.address, userId);
    const refreshed = await this.walletRepo.findByIdForUser(wallet.id, userId);
    return toWalletDto(refreshed!);
  }

  async remove(id: string, userId: string) {
    const result = await this.walletRepo.delete(id, userId);
    return result.count > 0;
  }

  async sync(id: string, userId: string) {
    const wallet = await this.walletRepo.findByIdForUser(id, userId);
    if (!wallet) return null;
    await this.syncAssets(wallet.id, wallet.address, userId);
    const refreshed = await this.walletRepo.findByIdForUser(id, userId);
    return refreshed ? toWalletDto(refreshed) : null;
  }

  private async syncAssets(walletId: string, address: string, userId: string) {
    const wallet = await this.walletRepo.findByIdForUser(walletId, userId);
    if (!wallet || wallet.chain !== "ETHEREUM") return;

    const summary = await this.onChainService.getSummary(address);
    const balance = summary.alchemy?.nativeBalance ?? summary.moralis?.nativeBalance;

    if (balance) {
      const ethAmount = (Number(balance) / 1e18).toString();
      await this.walletAssetRepo.upsert(walletId, "ETH", ethAmount, {
        isNative: true,
      });
    }
  }
}