"use client";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { createClient } from "@/lib/supabase/client";
import { SmoothScroll } from "../components/dashboard/smoothScroll";

export function Providers({ children }: { children: React.ReactNode }) {
  const userId = useRef<string | null | undefined>(undefined);
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            staleTime: 30_000, 
            refetchOnWindowFocus: false,
          },
        },
      })
  );

  useEffect(() => {
    if (!process.env.NEXT_PUBLIC_SUPABASE_URL || !process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY) return;
    const { data: { subscription } } = createClient().auth.onAuthStateChange((_event, session) => {
      const next = session?.user.id ?? null;
      if (userId.current !== undefined && userId.current !== next) {
        void queryClient.resetQueries({ queryKey: ["private"] });
      }
      userId.current = next;
    });
    return () => subscription.unsubscribe();
  }, [queryClient]);

  return (
    <QueryClientProvider client={queryClient}>
      <SmoothScroll>{children}</SmoothScroll>
    </QueryClientProvider>
  );
}
