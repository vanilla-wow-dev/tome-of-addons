// Typen + reine Helfer rund um die WoW-Erkennung.
// Bewusst frei von Vue/Tauri-Abhängigkeiten, damit isoliert testbar.

export interface WowRoot {
  path: string;
  has_exe: boolean;
  has_mpq: boolean;
  has_interface: boolean;
  has_addons: boolean;
  method: string;
}

export type ExeIdentity =
  | { status: "official"; version: string; locale: string }
  | { status: "modified"; claims_version: string }
  | { status: "unknown-build" }
  | { status: "unknown" };

export interface WowExeInfo {
  path: string;
  size_bytes: number;
  build: number | null;
  build_date: string | null;
  sha1: string;
  md5: string;
  identity: ExeIdentity;
  /**
   * Interface-Version des Clients (z. B. "11200"), sofern der Build bekannt ist.
   * Bezugswert für die Veraltet-Erkennung von Addons.
   */
  interface_version: string | null;
}

/**
 * Ergebnis der Erkennung. `managed` = der eindeutig verwaltete Root (Walk-up vom
 * Binary); `suggestions` = weitere erkannte Installationen, in die der Manager
 * verschoben werden kann. Siehe Determinismus-Prinzip im Rust-`wow.rs`.
 */
export interface Detection {
  managed: WowRoot | null;
  suggestions: WowRoot[];
}

/** Formatiert eine Byte-Zahl als B/KB/MB. */
export function fmtBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}
