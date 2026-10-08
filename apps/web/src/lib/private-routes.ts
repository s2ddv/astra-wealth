// Explicit per-domain contracts; no general-purpose authenticated proxy.
export function privateRoute(domain: string, path: string[], method: string): string | null {
  if (path.some((part) => !/^[a-zA-Z0-9][a-zA-Z0-9:_-]{0,199}$/.test(part))) return null;
  const [id, collection] = path;
  let allowed = false;
  if (domain === "wallets") {
    allowed = path.length === 0 ? ["GET", "POST"].includes(method)
      : path.length === 1 ? ["PATCH", "DELETE"].includes(method)
      : path.length === 2 && ((collection === "assets" && method === "GET") || (collection === "sync" && method === "POST"));
  } else if (domain === "accounts") {
    allowed = path.length === 0 ? ["GET", "POST"].includes(method)
      : path.length === 1 ? ["GET", "DELETE"].includes(method)
      : path.length === 2 ? ["holdings", "contributions"].includes(collection ?? "") && method === "POST"
      : path.length === 3 && ((collection === "holdings" && ["PUT", "DELETE"].includes(method)) || (collection === "contributions" && method === "DELETE"));
  } else if (domain === "watchlists") {
    allowed = path.length === 0 ? ["GET", "POST"].includes(method)
      : path.length === 1 ? method === "DELETE"
      : collection === "items" && ((path.length === 2 && method === "POST") || (path.length === 3 && method === "DELETE"));
  }
  if (!allowed || (path.length > 0 && !id)) return null;
  return `/v1/${domain === "accounts" ? "" : "me/"}${domain}${path.length ? `/${path.map(encodeURIComponent).join("/")}` : ""}`;
}
