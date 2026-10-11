export interface ManualOperation {
  id: string; accountId: string; requestId: string; kind: string;
  holdingId: string | null; destinationAccountId: string | null; replacesId: string | null;
  assetClass: string | null; market: string | null; quantity: string | null;
  amount: string | null; fees: string | null; currency: string; occurredAt: string;
  createdAt: string; note: string | null;
}
export interface OperationLedger {
  operations: ManualOperation[];
  results: { id: string; active: boolean; netAmount: string | null; assignedCost: string | null;
    realizedResult: string | null; unavailableReason: string | null }[];
  positions: { holdingId: string; quantity: string; accumulatedCost: string | null; averageCost: string | null }[];
}
