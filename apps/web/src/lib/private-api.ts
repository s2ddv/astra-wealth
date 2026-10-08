export class PrivateApiError extends Error {
  status: number;
  code: string | undefined;
  constructor(message: string, status: number, code?: string) {
    super(message);
    this.status = status;
    this.code = code;
  }
}

export async function privateFetch<T>(path: string, options: RequestInit = {}): Promise<T> {
  const response = await fetch(`/api/private/${path}`, {
    ...options, cache: "no-store", credentials: "same-origin",
    headers: { "Content-Type": "application/json", ...options.headers },
  });
  if (response.status === 204) return undefined as T;
  const body = await response.json();
  if (!response.ok) {
    const message = response.status === 401 ? "Entre na sua conta para continuar."
      : response.status === 429 ? "Aguarde alguns instantes antes de tentar novamente."
      : body.code === "CHAIN_NOT_SUPPORTED" ? "Sincronização indisponível para esta rede."
      : body.code === "WALLET_LINKED_TO_ACCOUNT" ? "Remova a conta vinculada antes de excluir esta carteira."
      : typeof body.error === "string" ? body.error : "Não foi possível concluir a operação.";
    throw new PrivateApiError(message, response.status, body.code);
  }
  return body as T;
}
