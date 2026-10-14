import { useSyncExternalStore } from "react";
import { setProviderSecret } from "@/lib/ipc/actions";
import { FIXTURE_ENDPOINTS, type EndpointRow, publicEndpoint } from "./model";

let endpoints: EndpointRow[] = FIXTURE_ENDPOINTS.map((row) => ({ ...row }));
let secret = "";
const listeners = new Set<() => void>();

function emit() {
  for (const listener of listeners) listener();
}

export function setEndpointSecret(value: string) {
  secret = value;
  endpoints = endpoints.map((row) => ({ ...row, hasSecret: value.length > 0 }));
  emit();
  void setProviderSecret("endpoint", value);
}

export function endpointSecret(): string {
  return secret;
}

export function exportEndpoints() {
  return endpoints.map((row) => publicEndpoint(row));
}

export function resetEndpoints() {
  endpoints = FIXTURE_ENDPOINTS.map((row) => ({ ...row }));
  secret = "";
  emit();
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function useEndpoints(): EndpointRow[] {
  return useSyncExternalStore(subscribe, () => endpoints, () => endpoints);
}
