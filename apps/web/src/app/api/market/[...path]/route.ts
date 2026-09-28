// Same-origin bridge to the Rust API. Provider credentials never enter the browser.
export async function GET(request: Request, context: { params: Promise<{ path: string[] }> }) {
  const { path } = await context.params;
  const allowed = (path.length === 1 && ["coins", "trending", "spot"].includes(path[0] ?? "")) ||
    (path.length === 2 && path[0] === "coins" && /^[a-z0-9][a-z0-9-]{0,99}$/.test(path[1] ?? ""));
  if (!allowed) return Response.json({ error: "Rota de mercado inválida." }, { status: 404 });
  const search = new URL(request.url).searchParams;
  if ([...search.keys()].some((key) => !["page", "perPage", "ids", "limit"].includes(key)) || search.toString().length > 11000) {
    return Response.json({ error: "Consulta inválida." }, { status: 400 });
  }
  try {
    const base = process.env.MARKET_API_URL ?? "http://127.0.0.1:3334";
    const url = new URL(`/v1/market/${path.map(encodeURIComponent).join("/")}`, base);
    url.search = search.toString();
    const response = await fetch(url, { cache: "no-store", redirect: "error", signal: AbortSignal.any([request.signal, AbortSignal.timeout(25000)]) });
    if (!response.ok) {
      return Response.json({ error: response.status === 404 ? "Ativo não encontrado." : "Dados de mercado indisponíveis. Tente novamente em instantes." }, { status: [400, 404, 429, 503].includes(response.status) ? response.status : 502, headers: response.headers.has("retry-after") ? { "Retry-After": "60" } : {} });
    }
    return Response.json(await response.json(), { headers: { "Cache-Control": "no-store" } });
  } catch {
    return Response.json({ error: "Não foi possível conectar à API de mercados." }, { status: 502 });
  }
}
