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

/** Menschlich lesbares Verdikt zur Exe-Identität. */
export function identityLabel(id: ExeIdentity): string {
  switch (id.status) {
    case "official":
      return `✓ Offiziell ${id.version} (${id.locale})`;
    case "modified":
      return `⚠ Modifiziert (gibt sich als ${id.claims_version} aus)`;
    case "unknown-build":
      return "⚠ Unbekannter Build (kein offizieller Referenz-Hash)";
    case "unknown":
      return "✗ Kein erkennbarer WoW-Client";
  }
}

/** Formatiert eine Byte-Zahl als B/KB/MB. */
export function fmtBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}
