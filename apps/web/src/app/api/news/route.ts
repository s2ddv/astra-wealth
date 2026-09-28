import { createClient } from "@/lib/supabase/server";

// Reuse the existing Supabase cookie session and the Rust same-origin bridge pattern.
export async function GET(request: Request) {
  const search = new URL(request.url).searchParams;
  if (search.toString().length > 512 || [...search.keys()].some((key) => !["category", "lang", "limit", "cursor"].includes(key))) {
    return Response.json({ error: "Consulta inválida." }, { status: 400 });
  }
  try {
    const supabase = await createClient();
    const { data: { session } } = await supabase.auth.getSession();
    if (!session) return Response.json({ error: "Entre na sua conta para ler as notícias." }, { status: 401 });
    const url = new URL("/v1/news", process.env.NEWS_API_URL ?? "http://127.0.0.1:3334");
    url.search = search.toString();
    // The Rust extractor verifies this JWT. Never forward arbitrary browser auth headers.
    const response = await fetch(url, {
      headers: { Authorization: `Bearer ${session.access_token}` }, cache: "no-store", redirect: "error",
      signal: AbortSignal.any([request.signal, AbortSignal.timeout(25000)]),
    });
    if (!response.ok) return Response.json({ error: "Não foi possível carregar as notícias." }, { status: [400, 401, 429, 503].includes(response.status) ? response.status : 502 });
    return Response.json(await response.json(), { headers: { "Cache-Control": "private, no-store" } });
  } catch {
    return Response.json({ error: "Não foi possível conectar ao serviço de notícias." }, { status: 502 });
  }
}
