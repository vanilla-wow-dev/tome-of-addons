import { createI18n } from "vue-i18n";
import en from "./locales/en";
import de from "./locales/de";
import fr from "./locales/fr";

export const SUPPORTED_LOCALES = ["en", "de", "fr"] as const;
export type Locale = (typeof SUPPORTED_LOCALES)[number];

const STORAGE_KEY = "locale";
const FALLBACK: Locale = "en";

// Bewusst window.localStorage (jsdoms/Browsers), nicht das bare `localStorage`
// — Letzteres kollidiert unter Node 22+ mit dessen experimentellem Global.
function storage(): Storage | undefined {
  return typeof window !== "undefined" ? window.localStorage : undefined;
}

function isSupported(value: string | null | undefined): value is Locale {
  return !!value && (SUPPORTED_LOCALES as readonly string[]).includes(value);
}

/**
 * Bestimmt die Startsprache: gespeicherte Wahl → Systemsprache → Fallback (en).
 * Aus `navigator.language` wird nur das primäre Subtag verwendet ("de-DE" → "de").
 */
export function detectLocale(): Locale {
  const stored = storage()?.getItem(STORAGE_KEY);
  if (isSupported(stored)) return stored;
  const system = (navigator.language || FALLBACK).toLowerCase().split("-")[0];
  return isSupported(system) ? system : FALLBACK;
}

export const i18n = createI18n({
  legacy: false,
  globalInjection: true,
  locale: detectLocale(),
  fallbackLocale: FALLBACK,
  messages: { en, de, fr },
});

/** Wechselt die Sprache zur Laufzeit und merkt sich die Wahl (localStorage). */
export function setLocale(locale: Locale): void {
  i18n.global.locale.value = locale;
  storage()?.setItem(STORAGE_KEY, locale);
  document.documentElement.lang = locale;
}
