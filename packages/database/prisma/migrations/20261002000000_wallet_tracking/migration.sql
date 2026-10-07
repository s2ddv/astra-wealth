BEGIN;
-- DropIndex
DROP INDEX "wallet_assets_walletId_symbol_key";

-- AlterTable
ALTER TABLE "wallet_assets" ADD COLUMN     "assetKey" TEXT,
ADD COLUMN     "decimals" INTEGER,
ALTER COLUMN "amount" SET DATA TYPE DECIMAL(400,255),
ALTER COLUMN "usdValue" SET DATA TYPE DECIMAL(600,355);

-- Preserve historical rows; a successful full sync replaces these legacy identities.
UPDATE wallet_assets SET "assetKey" = 'legacy:' || id;
ALTER TABLE wallet_assets ALTER COLUMN "assetKey" SET NOT NULL;

-- Normalize historical EVM addresses. The existing unique index rejects case
-- collisions transactionally; operators must resolve those explicitly first.
UPDATE wallets SET address = lower(address)
WHERE chain IN ('ETHEREUM','POLYGON','ARBITRUM','BASE');

-- CreateIndex
CREATE UNIQUE INDEX "wallet_assets_walletId_assetKey_key" ON "wallet_assets"("walletId", "assetKey");


ALTER TABLE wallets ADD COLUMN "lastSyncComplete" BOOLEAN NOT NULL DEFAULT false,
ADD COLUMN "lastValuationComplete" BOOLEAN NOT NULL DEFAULT false;
COMMIT;
