import { connection } from "next/server";
import { PortfolioHeader } from "@/components/dashboard/PortfolioHeader";

import { RealPortfolioOverview } from "@/components/dashboard/RealPortfolioOverview";

export default async function OverviewPage() {
  await connection();
  return process.env.PORTFOLIO_REAL_OVERVIEW_ENABLED === "true"
    ? <RealPortfolioOverview /> : <PortfolioHeader />;
}
