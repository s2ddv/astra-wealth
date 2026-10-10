CREATE TABLE "portfolio_valuations" (
    "id" TEXT NOT NULL,
    "userId" TEXT NOT NULL,
    "payload" JSONB NOT NULL,
    "createdAt" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "portfolio_valuations_pkey" PRIMARY KEY ("id"),
    CONSTRAINT "portfolio_valuations_userId_fkey" FOREIGN KEY ("userId") REFERENCES "users"("id") ON DELETE CASCADE ON UPDATE CASCADE
);
CREATE INDEX "portfolio_valuations_userId_createdAt_idx" ON "portfolio_valuations"("userId", "createdAt");
