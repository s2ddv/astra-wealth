import type {
  PortfolioSnapshot,
  Wallet,
  WalletAsset,
  Watchlist,
  WatchlistItem,
} from "@astra-wealth/database";
import type {
  PortfolioSnapshotDto,
  WalletAssetDto,
  WalletDto,
  WatchlistDto,
  WatchlistItemDto,
} from "@astra-wealth/shared";

export function toWalletAssetDto(asset: WalletAsset): WalletAssetDto {
  return {
    id: asset.id,
    symbol: asset.symbol,
    amount: asset.amount.toString(),
    walletId: asset.walletId,
    updatedAt: asset.updatedAt.toISOString(),
  };
}

export function toWalletDto(
  wallet: Wallet & { assets: WalletAsset[] }
): WalletDto {
  return {
    id: wallet.id,
    address: wallet.address,
    chain: wallet.chain as WalletDto["chain"],
    nickname: wallet.nickname,
    userId: wallet.userId,
    assets: wallet.assets.map(toWalletAssetDto),
    createdAt: wallet.createdAt.toISOString(),
    updatedAt: wallet.updatedAt.toISOString(),
  };
}

export function toWatchlistItemDto(item: WatchlistItem): WatchlistItemDto {
  return {
    id: item.id,
    coinId: item.coinId,
    addedAt: item.addedAt.toISOString(),
  };
}

export function toWatchlistDto(
  watchlist: Watchlist & { items: WatchlistItem[] }
): WatchlistDto {
  return {
    id: watchlist.id,
    name: watchlist.name,
    userId: watchlist.userId,
    items: watchlist.items.map(toWatchlistItemDto),
    createdAt: watchlist.createdAt.toISOString(),
    updatedAt: watchlist.updatedAt.toISOString(),
  };
}

export function toPortfolioSnapshotDto(
  snapshot: PortfolioSnapshot
): PortfolioSnapshotDto {
  return {
    id: snapshot.id,
    totalUsd: snapshot.totalUsd.toString(),
    userId: snapshot.userId,
    createdAt: snapshot.createdAt.toISOString(),
  };
}