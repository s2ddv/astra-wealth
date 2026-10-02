"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

const NAV_ITEMS = [
  { href: "/dashboard/accounts", label: "Contas", icon: "account_balance" },
  { href: "/dashboard/overview", label: "Overview", icon: "dashboard" },
  { href: "/dashboard/markets", label: "Markets", icon: "bar_chart" },
  { href: "/dashboard/wallets", label: "Wallets", icon: "account_balance_wallet" },
  { href: "/dashboard/news", label: "News", icon: "newspaper" },
  { href: "/dashboard/settings", label: "Settings", icon: "settings" },
];

interface SidebarProps {
  onAddAssets?: () => void;
  onLogout?: () => void;
}

export function Sidebar({ onAddAssets, onLogout }: SidebarProps) {
  const pathname = usePathname();

  return (
    <nav aria-label="Navegação principal" className="relative z-50 flex w-full flex-col justify-between border-b border-outline-variant bg-surface px-4 py-4 lg:fixed lg:left-0 lg:top-0 lg:h-full lg:w-[260px] lg:border-b-0 lg:border-r lg:py-8">
      <div className="flex min-w-0 flex-col gap-4 lg:gap-8">
        {/* Brand */}
        <div className="flex items-center gap-3 px-4">
          <div className="flex h-8 w-8 items-center justify-center rounded-full bg-primary font-bold text-on-primary">
            A
          </div>
          <div>
            <p className="text-headline-md font-bold text-on-surface">Astra</p>
            <p className="text-label-caps uppercase text-on-surface-variant opacity-60">
              Wealth Dashboard
            </p>
          </div>
        </div>

        {/* Main nav */}
        <ul className="flex gap-1 overflow-x-auto pb-1 lg:flex-col lg:overflow-visible lg:pb-0">
          {NAV_ITEMS.map((item) => {
            const isActive = pathname?.startsWith(item.href);
            return (
              <li key={item.href} className="shrink-0">
                <Link
                  href={item.href}
                  aria-current={isActive ? "page" : undefined}
                  className={`flex items-center gap-3 rounded-2xl px-4 py-3 focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-primary motion-safe:transition-colors duration-200 ${
                    isActive
                      ? "bg-primary/10 font-semibold text-primary"
                      : "text-on-surface-variant hover:bg-surface-variant/50 hover:text-on-surface"
                  }`}
                >
                  <span
                    aria-hidden="true" className="material-symbols-outlined"
                    style={
                      isActive ? { fontVariationSettings: "'FILL' 1" } : undefined
                    }
                  >
                    {item.icon}
                  </span>
                  <span className="text-body-base">{item.label}</span>
                </Link>
              </li>
            );
          })}
        </ul>
      </div>

      <div className="mt-auto hidden flex-col gap-4 lg:flex">
        <button
          onClick={onAddAssets}
          className="flex w-full items-center justify-center gap-2 rounded-full bg-primary px-4 py-3 font-medium text-on-primary transition-colors hover:bg-primary/90"
        >
          <span className="material-symbols-outlined text-[20px]">add</span>
          <span className="text-body-base">Add Assets</span>
        </button>

        <hr className="my-2 border-outline-variant/30" />

        <ul className="flex flex-col gap-1">
          <li>
            <Link
              href="/dashboard/settings#profile"
              className="flex items-center gap-3 rounded-2xl px-4 py-2 text-on-surface-variant transition-colors duration-200 hover:bg-surface-variant/50 hover:text-on-surface"
            >
              <span className="material-symbols-outlined">account_circle</span>
              <span className="text-body-sm">Profile</span>
            </Link>
          </li>
          <li>
            <button
              onClick={onLogout}
              className="flex w-full items-center gap-3 rounded-2xl px-4 py-2 text-on-surface-variant transition-colors duration-200 hover:bg-surface-variant/50 hover:text-on-surface"
            >
              <span className="material-symbols-outlined text-error">logout</span>
              <span className="text-body-sm text-error">Logout</span>
            </button>
          </li>
        </ul>
      </div>
    </nav>
  );
}