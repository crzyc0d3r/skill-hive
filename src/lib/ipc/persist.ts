import { invoke } from "@tauri-apps/api/core";
import { demoEnabled } from "../demo";

export function persistEffect(kind: string, id: string, body: string): void {
  if (demoEnabled()) return;
  void invoke("record_effect", { kind, id, body }).catch(() => undefined);
}
