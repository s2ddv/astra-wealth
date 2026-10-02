"use client";

import { usePathname } from "next/navigation";

import { AccountsProvider } from "./AccountsProvider";
import { Sidebar } from "./SideBar";
import { Topbar } from "./TopBar";
import { DashboardShell } from "./DashboardShell";

export function DashboardLayoutClient({
  children,
}: {
  children: React.ReactNode;
}) {
  const isOverview = usePathname() === "/dashboard/overview";
  return (
    <AccountsProvider>
      <div className="min-h-screen bg-background">
        <Sidebar />
        <DashboardShell>
          {!isOverview && <Topbar userName="Alex Rivera" />}
          <main className="px-4 pb-12 pt-6 sm:px-8">{children}</main>
        </DashboardShell>
      </div>
    </AccountsProvider>
  );
}
