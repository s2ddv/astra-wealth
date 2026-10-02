"use client";

import { AccountsProvider } from "./AccountsProvider";
import { Sidebar } from "./SideBar";
import { Topbar } from "./TopBar";
import { DashboardShell } from "./DashboardShell";

export function DashboardLayoutClient({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <AccountsProvider>
      <div className="min-h-screen bg-background">
        <Sidebar />
        <DashboardShell>
          <Topbar userName="Alex Rivera" />
          <div className="px-8 pb-12">{children}</div>
        </DashboardShell>
      </div>
    </AccountsProvider>
  );
}
