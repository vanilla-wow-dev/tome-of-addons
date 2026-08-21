import { describe, it, expect } from "vitest";
import {
  contrastRatio,
  readableColor,
  activeCount,
  installationHealth,
  stateFor,
  countOutdated,
  interfaceStatus,
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
    default_state: "disabled",
    file_count: 2310,
    size_bytes: 78_000_000,
    cached: false,
    error: null,
    ...overrides,
    // Segmente folgen dem tatsächlichen Titel, sonst zeigte die Tabelle den
    // Vorgabewert statt des überschriebenen.
    title_spans: overrides.title_spans ?? [
      { text: overrides.title ?? "pfQuest", color: null },
    ],
    // Ohne Farbcodes ist der rohe Titel der Anzeigetitel.
    title_raw: overrides.title_raw ?? overrides.title ?? "pfQuest",
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

describe("compareTitles – Reihenfolge des WoW-Clients", () => {
  /** Baut Addons aus rohen Titeln und sortiert sie wie der Client. */
  const sortRaw = (raws: string[]) =>
    raws
      .map((raw) => addon({ title_raw: raw }))
      .sort(compareTitles)
      .map((a) => a.title_raw);

  it("sortiert alphabetisch und ohne Rücksicht auf Groß-/Kleinschreibung", () => {
    expect(sortRaw(["Zed", "Atlas"])).toEqual(["Atlas", "Zed"]);
    expect(compareTitles(addon({ title_raw: "atlas" }), addon({ title_raw: "Atlas" }))).toBe(0);
  });

  it("reproduziert den Anfang der Client-Liste", () => {
    // Aus dem AddOn-Fenster abgelesen: „[K] …" steht vor „Accountant",
    // weil `[` (0x5B) vor `a` (0x61) liegt.
    expect(
      sortRaw([
        "AceTimer",
        "Accountant v2.3",
        "[K] Extended QuestLog 3.6.1",
        "Ace 1.3.1",
        "AceGUI",
      ]),
    ).toEqual([
      "[K] Extended QuestLog 3.6.1",
      "Accountant v2.3",
      "Ace 1.3.1",
      "AceGUI",
      "AceTimer",
    ]);
  });

  it("stellt alle gefärbten Titel hinter alle ungefärbten", () => {
    // `|` (0x7C) liegt hinter `z` (0x7A). Genau das sieht aus wie
    // „Farben beeinflussen die Reihenfolge".
    expect(
      sortRaw([
        "|cff33ffccShagu|cffffffffChat",
        "WIM",
        "|cFF006699Optional -|r AutoInvite",
        "XRaidStatus |cff7fff7f -Ace2-|r",
      ]),
    ).toEqual([
      "WIM",
      "XRaidStatus |cff7fff7f -Ace2-|r",
      "|cFF006699Optional -|r AutoInvite",
      "|cff33ffccShagu|cffffffffChat",
    ]);
  });

  it("reproduziert das Ende der Client-Liste über die Hex-Werte", () => {
    // Beobachtete Folge: 006699 (Optional/UUI) → 33ffcc (pf/Shagu)
    // → 3fcf26 ([mojo]) → b700b7 (Necrosis). Necrosis schreibt sein
    // `|CFF…` groß und ordnet sich trotzdem korrekt ein.
    expect(
      sortRaw([
        "|CFFB700B7N|CFFFF00FFecrosis LdC",
        "|cff3fcf26[mojo]|r Addons",
        "|cff33ffccShagu|cffffffffValue",
        "|cFF006699UUI -|r WhoPinged",
      ]),
    ).toEqual([
      "|cFF006699UUI -|r WhoPinged",
      "|cff33ffccShagu|cffffffffValue",
      "|cff3fcf26[mojo]|r Addons",
      "|CFFB700B7N|CFFFF00FFecrosis LdC",
    ]);
  });

  it("sortiert Zahlen wie der Client, also lexikalisch", () => {
    // Bewusst *keine* natürliche Sortierung: der Client hat sie nicht, und
    // Abweichen hieße, die Reihenfolge nicht mehr nachzubilden.
    expect(sortRaw(["Addon 2", "Addon 10"])).toEqual(["Addon 10", "Addon 2"]);
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

describe("interfaceStatus", () => {
  it("erkennt passende und abweichende Interface-Versionen", () => {
    expect(interfaceStatus(addon({ interface: "11200" }), "11200")).toBe("current");
    expect(interfaceStatus(addon({ interface: "11000" }), "11200")).toBe("outdated");
  });

  it("ignoriert umgebende Leerzeichen", () => {
    // "## Interface: 11200 " mit Trailing-Space kommt im echten Bestand vor.
    expect(interfaceStatus(addon({ interface: "11200 " }), "11200")).toBe("current");
  });

  it("behauptet nichts, wenn eine Seite fehlt", () => {
    expect(interfaceStatus(addon({ interface: null }), "11200")).toBe("unknown");
    expect(interfaceStatus(addon({ interface: "11200" }), null)).toBe("unknown");
  });
});

describe("countOutdated", () => {
  it("zählt nur die tatsächlich veralteten", () => {
    const addons = [
      addon({ interface: "11200" }),
      addon({ interface: "11000" }),
      addon({ interface: "11100" }),
      addon({ interface: null }),
    ];
    expect(countOutdated(addons, "11200")).toBe(2);
  });

  it("zählt nichts ohne bekannte Client-Version", () => {
    expect(countOutdated([addon({ interface: "11000" })], null)).toBe(0);
  });
});

describe("installationHealth", () => {
  const root = (has_addons: boolean) => ({
    path: "/games/WoW",
    has_exe: true,
    has_mpq: true,
    has_interface: true,
    has_addons,
    method: "walkup",
  });
  const exe = (status: string) =>
    ({
      path: "/games/WoW/WoW.exe",
      size_bytes: 1,
      build: 5875,
      build_date: null,
      sha1: "a",
      md5: "b",
      identity: { status } as never,
      interface_version: "11200",
    }) as never;

  it("ist rot ohne verwaltete Installation", () => {
    const verdict = installationHealth(null, null, "not-running");
    expect(verdict.level).toBe("error");
    expect(verdict.reasons).toEqual(["health.notManaged"]);
  });

  it("ist grün, wenn nichts dagegen spricht", () => {
    const verdict = installationHealth(root(true), exe("official"), "not-running");
    expect(verdict.level).toBe("ok");
    expect(verdict.reasons).toEqual([]);
  });

  it("warnt, solange der Client läuft", () => {
    // WoW schreibt SavedVariables beim Beenden zurück — Änderungen währenddessen
    // gehen verloren.
    expect(installationHealth(root(true), exe("official"), "running-here")).toEqual({
      level: "warn",
      reasons: ["health.clientRunningHere"],
    });
    expect(installationHealth(root(true), exe("official"), "running-unknown")).toEqual({
      level: "warn",
      reasons: ["health.clientRunningUnknown"],
    });
  });

  it("warnt bei fehlendem AddOns-Ordner", () => {
    expect(installationHealth(root(false), exe("official"), "not-running").reasons).toEqual([
      "health.noAddonsFolder",
    ]);
  });

  it("warnt bei jeder nicht-offiziellen Exe", () => {
    for (const status of ["modified", "unknown-build", "unknown"]) {
      expect(installationHealth(root(true), exe(status), "not-running").reasons).toEqual([
        `health.exe.${status}`,
      ]);
    }
  });

  it("nennt mehrere Gründe zugleich", () => {
    const verdict = installationHealth(root(false), exe("modified"), "running-here");
    expect(verdict.level).toBe("warn");
    expect(verdict.reasons).toEqual([
      "health.clientRunningHere",
      "health.noAddonsFolder",
      "health.exe.modified",
    ]);
  });

  it("urteilt ohne Exe-Analyse nicht über die Exe", () => {
    expect(installationHealth(root(true), null, "not-running").level).toBe("ok");
  });
});

describe("activeCount / stateFor", () => {
  const chr = (states: Record<string, boolean>) => ({
    account: "A",
    realm: "R",
    name: "C",
    path: "/p",
    label: "C · R (A)",
    states,
  });

  it("zählt nur installierte Addons", () => {
    // AddOns.txt führt gelöschte Addons weiter — sonst stünde in der
    // Seitenleiste eine Zahl, die zu keiner Liste passt.
    const addons = [addon({ id: "pfQuest" }), addon({ id: "Atlas" })];
    expect(activeCount(chr({ pfquest: true, geloescht: true }), addons)).toBe(1);
  });

  it("zählt abgeschaltete nicht mit", () => {
    expect(activeCount(chr({ pfquest: false }), [addon({ id: "pfQuest" })])).toBe(0);
  });

  it("unterscheidet nie gesehen von abgeschaltet", () => {
    const character = chr({ pfquest: false });
    expect(stateFor(character, addon({ id: "pfQuest" }))).toBe(false);
    expect(stateFor(character, addon({ id: "Unbekannt" }))).toBeNull();
    expect(stateFor(null, addon({ id: "pfQuest" }))).toBeNull();
  });
});

describe("Titelfarben", () => {
  const PANEL_LIGHT = "fbf6e9";
  const PANEL_DARK = "1f1811";

  it("berechnet das Kontrastverhältnis nach WCAG", () => {
    // Bekannte Eckwerte: Schwarz auf Weiß ist 21:1, gleiche Farbe 1:1.
    expect(contrastRatio("000000", "ffffff")).toBeCloseTo(21, 1);
    expect(contrastRatio("ff0000", "ff0000")).toBeCloseTo(1, 5);
    // Reihenfolge ist egal.
    expect(contrastRatio("000000", "ffffff")).toBeCloseTo(
      contrastRatio("ffffff", "000000"),
      5,
    );
  });

  it("lässt bereits lesbare Farben unangetastet", () => {
    // Dunkles Blau auf Pergament ist ohne Zutun kontraststark genug.
    expect(readableColor("003366", false)).toBe("#003366");
  });

  it("dunkelt zu helle Farben auf Pergament ab, bis sie lesbar sind", () => {
    // Reales Beispiel: |cffffffff aus „|cff33ffccShagu|cffffffffPlates".
    const adjusted = readableColor("ffffff", false);
    expect(adjusted).not.toBe("#ffffff");
    expect(contrastRatio(adjusted.slice(1), PANEL_LIGHT)).toBeGreaterThanOrEqual(4.5);
  });

  it("hellt zu dunkle Farben im Dunkelmodus auf", () => {
    const adjusted = readableColor("000000", true);
    expect(adjusted).not.toBe("#000000");
    expect(contrastRatio(adjusted.slice(1), PANEL_DARK)).toBeGreaterThanOrEqual(4.5);
  });

  it("erreicht den Mindestkontrast für alle Farben des echten Bestands", () => {
    // Aus den tatsächlich vorkommenden Titeln.
    const observed = ["33ffcc", "ffffff", "ff8080", "7fff7f", "006699", "3fcf26", "cfcfcf"];
    for (const dark of [false, true]) {
      for (const color of observed) {
        const adjusted = readableColor(color, dark).slice(1);
        expect(contrastRatio(adjusted, dark ? PANEL_DARK : PANEL_LIGHT)).toBeGreaterThanOrEqual(
          4.4,
        );
      }
    }
  });

  it("behält den Farbton beim Abdunkeln", () => {
    // Türkis bleibt türkis: Grün und Blau bleiben über Rot.
    const [r, g, b] = [1, 3, 5].map((i) => parseInt(readableColor("33ffcc", false).slice(i, i + 2), 16));
    expect(g).toBeGreaterThan(r);
    expect(b).toBeGreaterThan(r);
  });
});
