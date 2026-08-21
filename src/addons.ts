// Typen + reine Helfer rund um den Addon-Scan.
// Bewusst frei von Vue/Tauri-Abhängigkeiten, damit isoliert testbar.

import type { WowExeInfo, WowRoot } from "./wow";

export type AddonMode = "consumer" | "developer";

/** Ein Stück Titel mit eigener Farbe. */
export interface TitleSpan {
  text: string;
  /** `rrggbb`, oder `null` für die Standardfarbe. */
  color: string | null;
}

export interface Addon {
  id: string;
  path: string;
  title: string;
  /** Derselbe Titel in farbigen Abschnitten; leer bei Ersatznamen. */
  title_spans: TitleSpan[];
  version: string | null;
  interface: string | null;
  notes: string | null;
  author: string | null;
  tree_sha: string | null;
  tree_sha_short: string | null;
  mode: AddonMode;
  /** `## DefaultState` aus der .toc — nur der Anfangszustand, nicht der aktuelle. */
  default_state: string | null;
  file_count: number;
  size_bytes: number;
  cached: boolean;
  error: string | null;
}

export interface Character {
  account: string;
  realm: string;
  name: string;
  path: string;
  /** Fertige Anzeigeform „Charakter · Realm (Account)", vom Backend gesetzt. */
  label: string;
  /** Addon-Zustände dieses Charakters, Schlüssel kleingeschrieben. */
  states: Record<string, boolean>;
}

/** Addon-relevante Client-Einstellungen aus WTF/Config.wtf. */
export interface WowSettings {
  /** Lädt der Client Addons mit abweichender Interface-Version? */
  loads_outdated_addons: boolean;
}

/** Läuft gerade ein WoW-Client aus dieser Installation? */
export type ClientState = "not-running" | "running-here" | "running-unknown";

/** Ampel-Zustand der Installation. */
export type Health = "ok" | "warn" | "error";

export interface HealthVerdict {
  level: Health;
  /** i18n-Schlüssel der Gründe, in der Reihenfolge ihrer Schwere. */
  reasons: string[];
}

/**
 * Bewertet die verwaltete Installation für den Statuspunkt in der Seitenleiste.
 *
 * Ein laufender Client zählt als Warnung, obwohl er kein Defekt ist: WoW
 * schreibt die SavedVariables beim Beenden zurück und überschreibt damit alles,
 * was der Manager währenddessen ändert — und neue Addons sieht der Client erst
 * nach einem Neustart. Wer das nicht weiß, wundert sich über verschwundene
 * Änderungen.
 */
export function installationHealth(
  managed: WowRoot | null,
  exe: WowExeInfo | null | undefined,
  client: ClientState,
): HealthVerdict {
  if (!managed) return { level: "error", reasons: ["health.notManaged"] };

  const reasons: string[] = [];
  if (client === "running-here") reasons.push("health.clientRunningHere");
  else if (client === "running-unknown") reasons.push("health.clientRunningUnknown");
  if (!managed.has_addons) reasons.push("health.noAddonsFolder");
  if (exe && exe.identity.status !== "official") reasons.push(`health.exe.${exe.identity.status}`);

  return { level: reasons.length ? "warn" : "ok", reasons };
}

/**
 * Zählt die für einen Charakter aktiven Addons — aber nur solche, die auch
 * wirklich installiert sind.
 *
 * `AddOns.txt` führt Einträge weiter, deren Ordner längst gelöscht ist; ohne
 * diesen Abgleich stünde in der Seitenleiste eine Zahl, die zu keiner Liste passt.
 */
export function activeCount(character: Character, addons: Addon[]): number {
  return addons.filter((addon) => character.states[addon.id.toLowerCase()] === true).length;
}

/** Zustand eines Addons für einen Charakter; `null` = dem Client nie begegnet. */
export function stateFor(character: Character | null, addon: Addon): boolean | null {
  if (!character) return null;
  const value = character.states[addon.id.toLowerCase()];
  return value === undefined ? null : value;
}

export interface ScanProgress {
  done: number;
  total: number;
  current: string;
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

/* -------------------------------------------------------------------------
   Titelfarben

   Addon-Autoren färben ihre Titel für den dunklen Spiel-Hintergrund. `ffffff`
   auf Pergament wäre unsichtbar. Farben werden deshalb nur so weit
   nachgezogen, bis sie den Mindestkontrast erreichen — wer schon lesbar ist,
   bleibt unangetastet.
   ------------------------------------------------------------------------- */

/** Panel-Hintergründe aus `style.css` — Bezugspunkt für den Kontrast. */
const PANEL_LIGHT = "fbf6e9";
const PANEL_DARK = "1f1811";
/** WCAG-AA für normalen Text. */
const MIN_CONTRAST = 4.5;
/**
 * Zielwert beim Nachziehen. Etwas über der Schwelle, weil das Ergebnis auf
 * 8-Bit-Kanäle gerundet wird und sonst haarscharf darunter landen kann.
 */
const CONTRAST_TARGET = 4.6;

function channels(hex: string): [number, number, number] {
  const value = parseInt(hex, 16);
  return [(value >> 16) & 255, (value >> 8) & 255, value & 255];
}

function toLinear(channel: number): number {
  const c = channel / 255;
  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

function toSrgb(linear: number): number {
  const c = linear <= 0.0031308 ? linear * 12.92 : 1.055 * linear ** (1 / 2.4) - 0.055;
  return Math.round(Math.min(1, Math.max(0, c)) * 255);
}

/** `rrggbb` in lineares Licht. Getrennt von der Luminanz, damit nicht
 *  versehentlich zweimal umgerechnet wird. */
function linearOf(hex: string): [number, number, number] {
  return channels(hex).map(toLinear) as [number, number, number];
}

/** Relative Luminanz aus *linearen* Kanälen. */
function luminanceOf([r, g, b]: [number, number, number]): number {
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** Kontrastverhältnis zweier Farben nach WCAG, jeweils als `rrggbb`. */
export function contrastRatio(a: string, b: string): number {
  const [high, low] = [luminanceOf(linearOf(a)), luminanceOf(linearOf(b))].sort((x, y) => y - x);
  return (high + 0.05) / (low + 0.05);
}

/**
 * Zieht eine Autorenfarbe so weit nach, dass sie vor dem Panel lesbar bleibt.
 *
 * Abgedunkelt wird durch Skalieren im linearen Licht (der Farbton bleibt),
 * aufgehellt durch Mischen mit Weiß — Skalieren nach oben würde Kanäle in die
 * Sättigung treiben und den Farbton verziehen.
 */
export function readableColor(hex: string, dark: boolean): string {
  const background = dark ? PANEL_DARK : PANEL_LIGHT;
  if (contrastRatio(hex, background) >= MIN_CONTRAST) return `#${hex}`;

  const backgroundLuminance = luminanceOf(linearOf(background));
  const linear = linearOf(hex);
  const current = luminanceOf(linear);

  let adjusted: [number, number, number];
  if (dark) {
    const target = (backgroundLuminance + 0.05) * CONTRAST_TARGET - 0.05;
    const mix = Math.min(1, Math.max(0, (target - current) / (1 - current)));
    adjusted = linear.map((c) => c + (1 - c) * mix) as [number, number, number];
  } else {
    const target = (backgroundLuminance + 0.05) / CONTRAST_TARGET - 0.05;
    const scale = current > 0 ? Math.min(1, target / current) : 1;
    adjusted = linear.map((c) => c * scale) as [number, number, number];
  }
  return `#${adjusted.map((c) => toSrgb(c).toString(16).padStart(2, "0")).join("")}`;
}

/** Verhältnis eines Addons zur Interface-Version des Clients. */
export type InterfaceStatus = "current" | "outdated" | "unknown";

/**
 * Vergleicht `## Interface` des Addons mit der Interface-Version des Clients.
 *
 * Das ist keine Kosmetik: WoW lädt Addons mit abweichender Interface-Version
 * nur, wenn „Veraltete AddOns laden" angehakt ist. Ein veraltetes Addon fehlt
 * sonst wortlos im Spiel.
 *
 * `unknown`, wenn eine der beiden Seiten fehlt — dann wird nichts behauptet.
 */
export function interfaceStatus(addon: Addon, clientInterface: string | null): InterfaceStatus {
  if (!clientInterface || !addon.interface) return "unknown";
  return addon.interface.trim() === clientInterface.trim() ? "current" : "outdated";
}

/** Anzahl der Addons, die der Client nicht ohne Weiteres lädt. */
export function countOutdated(addons: Addon[], clientInterface: string | null): number {
  return addons.filter((addon) => interfaceStatus(addon, clientInterface) === "outdated").length;
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
