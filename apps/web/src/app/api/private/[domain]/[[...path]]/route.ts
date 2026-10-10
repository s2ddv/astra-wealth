import { createClient } from "@/lib/supabase/server";
import { privateRoute } from "@/lib/private-routes";

const headers = { "Cache-Control": "private, no-store" };
async function handle(request: Request, context: { params: Promise<{ domain: string; path?: string[] }> }) {
  const { domain, path = [] } = await context.params;
  const route = privateRoute(domain, path, request.method);
  const url = new URL(request.url);
  if (!route || url.search) return Response.json({ error: "Rota inválida." }, { status: 404, headers });
  if (request.method !== "GET" && request.headers.get("origin") !== url.origin) {
    return Response.json({ error: "Origem inválida." }, { status: 403, headers });
  }
  try {
    const supabase = await createClient();
    const { data: { session } } = await supabase.auth.getSession();
    if (!session) return Response.json({ error: "Sessão necessária." }, { status: 401, headers });
    let body: string | undefined;
    if (["POST", "PUT", "PATCH"].includes(request.method)) {
      if (!request.headers.get("content-type")?.startsWith("application/json")) {
        return Response.json({ error: "Envie JSON." }, { status: 415, headers });
      }
      body = await request.text();
      if (new TextEncoder().encode(body).length > 65536) return Response.json({ error: "Corpo muito grande." }, { status: 413, headers });
    }
    const bases: Record<string, string | undefined> = {
      wallets: process.env.WALLETS_API_URL, accounts: process.env.ACCOUNTS_API_URL,
      watchlists: process.env.WATCHLISTS_API_URL,
      portfolio: process.env.PORTFOLIO_API_URL, fx: process.env.PORTFOLIO_API_URL,
    };
    // getSession supplies only the bearer; Rust verifies signature, user and ownership.
    const response = await fetch(new URL(route, bases[domain] ?? "http://127.0.0.1:3334"), {
      method: request.method, ...(body === undefined ? {} : { body }), cache: "no-store", redirect: "error",
      headers: { Authorization: `Bearer ${session.access_token}`, "Content-Type": "application/json" },
      signal: AbortSignal.any([request.signal, AbortSignal.timeout(65000)]),
    });
    if (response.status === 204) return new Response(null, { status: 204, headers });
    const result = await response.json();
    return Response.json(result, { status: response.status, headers: {
      ...headers, ...(response.status === 429 ? { "Retry-After": "15" } : {}),
    } });
  } catch {
    return Response.json({ error: "Serviço indisponível. Tente novamente." }, { status: 502, headers });
  }
}
export { handle as GET, handle as POST, handle as PUT, handle as PATCH, handle as DELETE };
