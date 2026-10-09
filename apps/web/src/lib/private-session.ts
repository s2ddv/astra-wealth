import { QueryClient } from "@tanstack/react-query";
import { privateFetch, PrivateApiError } from "./private-api.ts";

function createScope(userId: string | null | undefined, generation: number) {
  const controller = new AbortController();
  const client = new QueryClient({
    defaultOptions: { queries: { staleTime: 30_000, refetchOnWindowFocus: false } },
  });
  return {
    userId,
    generation,
    client,
    get disposed() { return controller.signal.aborted; },
    // Each operation captures its originating scope, including follow-up requests.
    async fetch<T>(path: string, options: RequestInit = {}): Promise<T> {
      const signal = options.signal
        ? AbortSignal.any([controller.signal, options.signal]) : controller.signal;
      signal.throwIfAborted();
      if (!userId) throw new PrivateApiError("Entre na sua conta para continuar.", 401);
      const result = await privateFetch<T>(path, { ...options, signal });
      // A response/body parser may finish after logout even when abort is ignored.
      signal.throwIfAborted();
      return result;
    },
    dispose() {
      controller.abort();
      client.clear();
    },
  };
}

export type PrivateSession = ReturnType<typeof createScope>;

export function createPrivateSessionCache() {
  // Unknown identity may render public UI, but cannot start private requests.
  let current = createScope(undefined, 0);
  return {
    get current() { return current; },
    switchTo(userId: string | null) {
      if (current.userId === userId && !current.disposed) return current;
      current.dispose();
      current = createScope(userId, current.generation + 1);
      return current;
    },
    dispose() { current.dispose(); },
  };
}
