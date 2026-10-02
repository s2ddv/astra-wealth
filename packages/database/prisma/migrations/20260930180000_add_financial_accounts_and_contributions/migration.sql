-- CreateEnum
CREATE TYPE "AccountKind" AS ENUM ('WALLET', 'EXCHANGE', 'BANK', 'BROKERAGE', 'MANUAL');

-- CreateEnum
CREATE TYPE "AssetClass" AS ENUM ('CRYPTO', 'STOCK', 'REAL_ESTATE_FUND', 'ETF', 'FIXED_INCOME', 'MUTUAL_FUND', 'PENSION', 'CASH', 'OTHER');

-- CreateEnum
CREATE TYPE "ContributionKind" AS ENUM ('DEPOSIT', 'WITHDRAWAL');

-- CreateEnum
CREATE TYPE "DataOrigin" AS ENUM ('MANUAL', 'IMPORTED');

-- CreateEnum
CREATE TYPE "FiatCurrency" AS ENUM ('BRL', 'USD');

-- CreateEnum
CREATE TYPE "OpenFinanceProvider" AS ENUM ('PLUGGY');

-- CreateEnum
CREATE TYPE "ConnectionStatus" AS ENUM ('ACTIVE', 'NEEDS_REAUTH', 'EXPIRED', 'ERROR');

-- CreateTable
CREATE TABLE "OpenFinanceConnection" (
    "id" TEXT NOT NULL,
    "userId" TEXT NOT NULL,
    "provider" "OpenFinanceProvider" NOT NULL,
    "externalItemId" TEXT NOT NULL,
    "institutionName" TEXT NOT NULL,
    "status" "ConnectionStatus" NOT NULL DEFAULT 'ACTIVE',
    "consentExpiresAt" TIMESTAMP(3),
    "lastSyncedAt" TIMESTAMP(3),
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "OpenFinanceConnection_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "FinancialAccount" (
    "id" TEXT NOT NULL,
    "userId" TEXT NOT NULL,
    "kind" "AccountKind" NOT NULL,
    "name" TEXT NOT NULL,
    "institutionName" TEXT,
    "baseCurrency" "FiatCurrency" NOT NULL DEFAULT 'BRL',
    "walletId" TEXT,
    "exchangeConnectionId" TEXT,
    "openFinanceConnectionId" TEXT,
    "externalAccountId" TEXT,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP(3) NOT NULL,

    CONSTRAINT "FinancialAccount_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "Holding" (
    "id" TEXT NOT NULL,
    "accountId" TEXT NOT NULL,
    "assetClass" "AssetClass" NOT NULL,
    "symbol" TEXT,
    "name" TEXT NOT NULL,
    "quantity" DECIMAL(28,10) NOT NULL,
    "unitPrice" DECIMAL(28,10),
    "currentValue" DECIMAL(28,10) NOT NULL,
    "dueDate" TIMESTAMP(3),
    "origin" "DataOrigin" NOT NULL DEFAULT 'MANUAL',
    "externalId" TEXT,
    "updatedAt" TIMESTAMP(3) NOT NULL,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "Holding_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "Contribution" (
    "id" TEXT NOT NULL,
    "accountId" TEXT NOT NULL,
    "kind" "ContributionKind" NOT NULL,
    "amount" DECIMAL(28,10) NOT NULL,
    "currency" "FiatCurrency" NOT NULL,
    "fxRateToBrl" DECIMAL(18,8),
    "occurredAt" TIMESTAMP(3) NOT NULL,
    "note" TEXT,
    "origin" "DataOrigin" NOT NULL DEFAULT 'MANUAL',
    "externalId" TEXT,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "Contribution_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE INDEX "OpenFinanceConnection_userId_idx" ON "OpenFinanceConnection"("userId");

-- CreateIndex
CREATE UNIQUE INDEX "OpenFinanceConnection_provider_externalItemId_key" ON "OpenFinanceConnection"("provider", "externalItemId");

-- CreateIndex
CREATE UNIQUE INDEX "FinancialAccount_walletId_key" ON "FinancialAccount"("walletId");

-- CreateIndex
CREATE UNIQUE INDEX "FinancialAccount_exchangeConnectionId_key" ON "FinancialAccount"("exchangeConnectionId");

-- CreateIndex
CREATE INDEX "FinancialAccount_userId_idx" ON "FinancialAccount"("userId");

-- CreateIndex
CREATE INDEX "Holding_accountId_idx" ON "Holding"("accountId");

-- CreateIndex
CREATE UNIQUE INDEX "Holding_accountId_externalId_key" ON "Holding"("accountId", "externalId");

-- CreateIndex
CREATE INDEX "Contribution_accountId_occurredAt_idx" ON "Contribution"("accountId", "occurredAt");

-- CreateIndex
CREATE UNIQUE INDEX "Contribution_accountId_externalId_key" ON "Contribution"("accountId", "externalId");

-- AddForeignKey
ALTER TABLE "OpenFinanceConnection" ADD CONSTRAINT "OpenFinanceConnection_userId_fkey" FOREIGN KEY ("userId") REFERENCES "users"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "FinancialAccount" ADD CONSTRAINT "FinancialAccount_userId_fkey" FOREIGN KEY ("userId") REFERENCES "users"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "FinancialAccount" ADD CONSTRAINT "FinancialAccount_walletId_fkey" FOREIGN KEY ("walletId") REFERENCES "wallets"("id") ON DELETE SET NULL ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "FinancialAccount" ADD CONSTRAINT "FinancialAccount_exchangeConnectionId_fkey" FOREIGN KEY ("exchangeConnectionId") REFERENCES "exchange_connections"("id") ON DELETE SET NULL ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "FinancialAccount" ADD CONSTRAINT "FinancialAccount_openFinanceConnectionId_fkey" FOREIGN KEY ("openFinanceConnectionId") REFERENCES "OpenFinanceConnection"("id") ON DELETE SET NULL ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "Holding" ADD CONSTRAINT "Holding_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "FinancialAccount"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "Contribution" ADD CONSTRAINT "Contribution_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "FinancialAccount"("id") ON DELETE CASCADE ON UPDATE CASCADE;

