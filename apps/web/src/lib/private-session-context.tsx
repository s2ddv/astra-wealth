"use client";

import { createContext, useContext } from "react";
import type { PrivateSession } from "./private-session";

export const PrivateSessionContext = createContext<PrivateSession["fetch"] | null>(null);

export function usePrivateFetch() {
  const request = useContext(PrivateSessionContext);
  if (!request) throw new Error("PrivateSessionContext não encontrado.");
  return request;
}
