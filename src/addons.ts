// Typen + reine Helfer rund um den Addon-Scan.
// Bewusst frei von Vue/Tauri-Abhängigkeiten, damit isoliert testbar.

export type AddonMode = "consumer" | "developer";

export interface Addon {
  id: string;
  path: string;
  title: string;
  version: string | null;
  interface: string | null;
  notes: string | null;
  author: string | null;
  tree_sha: string | null;
  tree_sha_short: string | null;
  mode: AddonMode;
  file_count: number;
  size_bytes: number;
  cached: boolean;
  error: string | null;
}

export interface SkippedFolder {
  id: string;
  reason: string;
}

export interface AddonScan {
  addons: Addon[];
  skipped: SkippedFolder[];
  cache_hits: number;
  hashed: number;
}

/**
 * Freitextsuche über ein Addon.
 *
 * Durchsucht bewusst auch Hash und Pfad: Ein Nutzer, der einen Tree-Hash aus
 * einer Fehlermeldung oder dem Index kopiert, will damit sein Addon finden.
 * Die Suche ist case-insensitiv und ignoriert umgebende Leerzeichen.
 */
export function matchesQuery(addon: Addon, query: string): boolean {
  const needle = query.trim().toLowerCase();
  if (!needle) return true;
  return [
    addon.id,
    addon.title,
    addon.author,
    addon.version,
    addon.notes,
    addon.tree_sha,
    addon.path,
  ].some((field) => !!field && field.toLowerCase().includes(needle));
}

/**
 * Sortierwert für die Addon-Spalte.
 *
 * Sortiert wird nach dem Anzeigetitel, nicht nach der Ordner-ID — das ist, was
 * der Nutzer liest. `localeCompare` mit `numeric`, damit "Addon 2" vor
 * "Addon 10" landet, und `sensitivity: "base"`, damit Groß-/Kleinschreibung und
 * Akzente die Reihenfolge nicht zerreißen.
 */
export function compareTitles(a: Addon, b: Addon): number {
  return a.title.localeCompare(b.title, undefined, {
    numeric: true,
    sensitivity: "base",
  });
}

/** Formatiert eine Dateianzahl mit Tausendertrennung der aktiven Locale. */
export function fmtCount(n: number): string {
  return n.toLocaleString();
}

/**
 * Kurzform des Tree-Hash für die Tabelle. Fehlt der Hash (Hashen
 * fehlgeschlagen), liefert sie einen Platzhalter statt eines leeren Feldes —
 * eine leere Zelle wäre von "noch nicht berechnet" nicht zu unterscheiden.
 */
export function shortHash(addon: Addon): string {
  return addon.tree_sha_short ?? "—";
}

/** Anteil der Addons, deren Hash aus dem Cache kam. */
export function cacheRatio(scan: AddonScan): number {
  const total = scan.cache_hits + scan.hashed;
  return total === 0 ? 0 : scan.cache_hits / total;
}
