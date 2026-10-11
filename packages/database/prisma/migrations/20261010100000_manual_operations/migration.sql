CREATE TABLE "financial_operations" (
 "id" TEXT PRIMARY KEY, "accountId" TEXT NOT NULL, "destinationAccountId" TEXT,
 "holdingId" TEXT, "replacesId" TEXT, "requestId" TEXT NOT NULL,
 "kind" TEXT NOT NULL CHECK ("kind" IN ('BUY','SELL','INCOME','FEE','TRANSFER','OPENING')),
 "assetClass" TEXT, "market" TEXT CHECK ("market" IN ('BR','INTERNATIONAL','UNKNOWN')),
 "quantity" DECIMAL(28,10) CHECK ("quantity" > 0),
 "amount" DECIMAL(28,10) CHECK ("amount" >= 0),
 "fees" DECIMAL(28,10) CHECK ("fees" >= 0),
 "currency" "FiatCurrency" NOT NULL, "occurredAt" TIMESTAMP(3) NOT NULL,
 "note" TEXT, "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
 CONSTRAINT "financial_operations_accountId_fkey" FOREIGN KEY ("accountId") REFERENCES "FinancialAccount"("id") ON DELETE RESTRICT ON UPDATE CASCADE,
 CONSTRAINT "financial_operations_destinationAccountId_fkey" FOREIGN KEY ("destinationAccountId") REFERENCES "FinancialAccount"("id") ON DELETE RESTRICT ON UPDATE CASCADE,
 CONSTRAINT "financial_operations_holdingId_fkey" FOREIGN KEY ("holdingId") REFERENCES "Holding"("id") ON DELETE RESTRICT ON UPDATE CASCADE,
 CONSTRAINT "financial_operations_replacesId_fkey" FOREIGN KEY ("replacesId") REFERENCES "financial_operations"("id") ON DELETE RESTRICT ON UPDATE CASCADE,
 CHECK ("destinationAccountId" IS NULL OR "destinationAccountId" <> "accountId")
);
CREATE UNIQUE INDEX "financial_operations_replacesId_key" ON "financial_operations"("replacesId");
CREATE UNIQUE INDEX "financial_operations_accountId_requestId_key" ON "financial_operations"("accountId", "requestId");
CREATE INDEX "financial_operations_accountId_occurredAt_idx" ON "financial_operations"("accountId", "occurredAt");
CREATE INDEX "financial_operations_destinationAccountId_occurredAt_idx" ON "financial_operations"("destinationAccountId", "occurredAt");
CREATE INDEX "financial_operations_holdingId_idx" ON "financial_operations"("holdingId");
-- The ledger is served only by the Rust backend (table owner/BYPASSRLS).
-- No browser-role policies: default grants must never expose financial records.
ALTER TABLE "financial_operations" ENABLE ROW LEVEL SECURITY;
