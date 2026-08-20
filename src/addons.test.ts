import { describe, it, expect } from "vitest";
import {
  cacheRatio,
  compareTitles,
  fmtCount,
  matchesQuery,
  shortHash,
  type Addon,
  type AddonScan,
} from "./addons";

function addon(overrides: Partial<Addon> = {}): Addon {
  return {
    id: "pfQuest",
    path: "/games/WoW/Interface/AddOns/pfQuest",
    title: "pfQuest",
    version: "GIT",
    interface: "11200",
    notes: "A lightweight Questhelper",
    author: "Shagu",
    tree_sha: "7c1d90ffaa11223344556677889900aabbccddeeff00112233445566778899aa",
    tree_sha_short: "7c1d90ffaa11",
    mode: "consumer",
    file_count: 2310,
    size_bytes: 78_000_000,
    cached: false,
    error: null,
    ...overrides,
  };
}

describe("matchesQuery", () => {
  it("lässt bei leerer Suche alles durch", () => {
    expect(matchesQuery(addon(), "")).toBe(true);
    expect(matchesQuery(addon(), "   ")).toBe(true);
  });

  it("sucht case-insensitiv in Titel, ID und Autor", () => {
    expect(matchesQuery(addon(), "PFQUEST")).toBe(true);
    expect(matchesQuery(addon(), "shagu")).toBe(true);
    expect(matchesQuery(addon(), "questhelper")).toBe(true);
  });

  it("findet ein Addon über seinen Tree-Hash", () => {
    // Der Fall zählt: Hashes werden aus Fehlermeldungen und dem Index kopiert.
    expect(matchesQuery(addon(), "7c1d90ffaa11")).toBe(true);
    expect(matchesQuery(addon(), "8899aa")).toBe(true);
  });

  it("sucht auch im Pfad", () => {
    expect(matchesQuery(addon(), "Interface/AddOns")).toBe(true);
  });

  it("ignoriert umgebende Leerzeichen der Eingabe", () => {
    expect(matchesQuery(addon(), "  shagu  ")).toBe(true);
  });

  it("verträgt fehlende Felder", () => {
    const bare = addon({ version: null, notes: null, author: null, tree_sha: null });
    expect(matchesQuery(bare, "shagu")).toBe(false);
    expect(matchesQuery(bare, "pfQuest")).toBe(true);
  });

  it("liefert false, wenn nichts passt", () => {
    expect(matchesQuery(addon(), "bagnon")).toBe(false);
  });
});

describe("compareTitles", () => {
  it("sortiert alphabetisch nach dem Anzeigetitel", () => {
    const sorted = [addon({ title: "Zed" }), addon({ title: "Atlas" })].sort(compareTitles);
    expect(sorted.map((a) => a.title)).toEqual(["Atlas", "Zed"]);
  });

  it("ignoriert Groß-/Kleinschreibung", () => {
    expect(compareTitles(addon({ title: "atlas" }), addon({ title: "Atlas" }))).toBe(0);
  });

  it("sortiert Zahlen natürlich statt lexikalisch", () => {
    // Ohne `numeric` landete "Addon 10" vor "Addon 2".
    const sorted = [addon({ title: "Addon 10" }), addon({ title: "Addon 2" })].sort(compareTitles);
    expect(sorted.map((a) => a.title)).toEqual(["Addon 2", "Addon 10"]);
  });
});

describe("shortHash", () => {
  it("gibt die Kurzform zurück", () => {
    expect(shortHash(addon())).toBe("7c1d90ffaa11");
  });

  it("zeigt einen Platzhalter, wenn das Hashen fehlschlug", () => {
    // Eine leere Zelle wäre von "noch nicht berechnet" nicht zu unterscheiden.
    expect(shortHash(addon({ tree_sha_short: null }))).toBe("—");
  });
});

describe("fmtCount", () => {
  it("formatiert mit Tausendertrennung", () => {
    expect(fmtCount(2310)).toBe((2310).toLocaleString());
    expect(fmtCount(0)).toBe("0");
  });
});

describe("cacheRatio", () => {
  const scan = (cache_hits: number, hashed: number): AddonScan => ({
    addons: [],
    skipped: [],
    cache_hits,
    hashed,
  });

  it("berechnet den Cache-Anteil", () => {
    expect(cacheRatio(scan(3, 1))).toBe(0.75);
    expect(cacheRatio(scan(242, 0))).toBe(1);
  });

  it("liefert 0 statt NaN bei leerem Scan", () => {
    expect(cacheRatio(scan(0, 0))).toBe(0);
  });
});
