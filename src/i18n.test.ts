import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { detectLocale, setLocale, i18n, SUPPORTED_LOCALES } from "./i18n";
import en from "./locales/en";
import de from "./locales/de";
import fr from "./locales/fr";

/** Überschreibt navigator.language (eigene Property schattet den Prototyp-Getter). */
function setNavigatorLanguage(lang: string) {
  Object.defineProperty(navigator, "language", { value: lang, configurable: true });
}

beforeEach(() => window.localStorage.clear());
afterEach(() => {
  // @ts-expect-error eigene Property entfernen → Prototyp-Getter greift wieder
  delete navigator.language;
});

describe("detectLocale", () => {
  it("nimmt die gespeicherte Sprache, wenn gültig", () => {
    window.localStorage.setItem("locale", "fr");
    expect(detectLocale()).toBe("fr");
  });

  it("ignoriert eine ungültige gespeicherte Sprache und nutzt die Systemsprache", () => {
    window.localStorage.setItem("locale", "xx");
    setNavigatorLanguage("de-DE");
    expect(detectLocale()).toBe("de");
  });

  it("erkennt die Systemsprache ohne gespeicherte Wahl", () => {
    setNavigatorLanguage("fr-CA");
    expect(detectLocale()).toBe("fr");
  });

  it("fällt auf Englisch zurück bei nicht unterstützter Systemsprache", () => {
    setNavigatorLanguage("es-ES");
    expect(detectLocale()).toBe("en");
  });

  it("fällt auf Englisch zurück, wenn keine Systemsprache vorliegt", () => {
    setNavigatorLanguage("");
    expect(detectLocale()).toBe("en");
  });
});

describe("setLocale", () => {
  it("setzt die aktive Sprache und persistiert sie", () => {
    setLocale("fr");
    expect(i18n.global.locale.value).toBe("fr");
    expect(window.localStorage.getItem("locale")).toBe("fr");
    expect(document.documentElement.lang).toBe("fr");
  });
});

describe("Übersetzungen", () => {
  const keys = (obj: Record<string, unknown>, prefix = ""): string[] =>
    Object.entries(obj).flatMap(([k, v]) =>
      v && typeof v === "object"
        ? keys(v as Record<string, unknown>, `${prefix}${k}.`)
        : [`${prefix}${k}`],
    );

  it("alle Locales haben exakt dieselben Schlüssel", () => {
    const enKeys = keys(en).sort();
    expect(keys(de).sort()).toEqual(enKeys);
    expect(keys(fr).sort()).toEqual(enKeys);
  });

  it("unterstützt genau en/de/fr", () => {
    expect([...SUPPORTED_LOCALES]).toEqual(["en", "de", "fr"]);
  });
});
