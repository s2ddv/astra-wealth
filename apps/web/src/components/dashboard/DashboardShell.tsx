export function DashboardShell({ children }: { children: React.ReactNode }) {
  return (
    <div className="lg:ml-[260px] min-h-screen bg-background text-on-surface">
      {children}
    </div>
  );
}
