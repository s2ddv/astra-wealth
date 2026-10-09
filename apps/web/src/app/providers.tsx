"use client";

import { QueryClientProvider } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { createClient } from "@/lib/supabase/client";
import { createPrivateSessionCache } from "@/lib/private-session";
import { PrivateSessionContext } from "@/lib/private-session-context";
import { SmoothScroll } from "../components/dashboard/smoothScroll";

export function Providers({ children }: { children: React.ReactNode }) {
  const [sessions] = useState(createPrivateSessionCache);
  const [scope, setScope] = useState(() => sessions.current);

  useEffect(() => {
    if (!process.env.NEXT_PUBLIC_SUPABASE_URL || !process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY) {
      setScope(sessions.switchTo(null));
      return () => sessions.dispose();
    }
    const { data: { subscription } } = createClient().auth.onAuthStateChange((_event, session) => {
      setScope(sessions.switchTo(session?.user.id ?? null));
    });
    return () => {
      subscription.unsubscribe();
      sessions.dispose();
    };
  }, [sessions]);

  return (
    <SmoothScroll>
      {/* Remount observers and local state; late callbacks retain the retired client. */}
      <QueryClientProvider key={scope.generation} client={scope.client}>
        <PrivateSessionContext.Provider value={scope.fetch}>{children}</PrivateSessionContext.Provider>
      </QueryClientProvider>
    </SmoothScroll>
  );
}
