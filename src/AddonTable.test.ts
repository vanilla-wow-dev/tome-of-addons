import { describe, it, expect, beforeEach, vi } from "vitest";
import { mount, config, type VueWrapper } from "@vue/test-utils";
import { i18n, setLocale } from "./i18n";
import type { Addon, AddonScan, Character } from "./addons";
import AddonTable from "./AddonTable.vue";

config.global.plugins = [i18n];

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

const SCAN: AddonScan = {
  addons: [
    addon({ id: "pfQuest", title: "pfQuest" }),
    addon({
      id: "Invite-o-matik",
      title: "Invite-o-matik",
      author: "Ulf",
      version: "2.1",
      mode: "developer",
      tree_sha_short: "a91f0c33bb22",
      tree_sha: "a91f0c33bb22" + "0".repeat(52),
      file_count: 1204,
      size_bytes: 80_000_000,
      cached: true,
    }),
    addon({
      id: "Accountant",
      title: "Accountant",
      author: null,
      version: null,
      notes: null,
      // Eigener Hash, damit Sortiertests nicht am Tie-Break hängen.
      tree_sha_short: "1a2b3c4d5e6f",
      tree_sha: "1a2b3c4d5e6f" + "0".repeat(52),
      file_count: 14,
      size_bytes: 120_000,
    }),
  ],
  skipped: [
    { id: "BlizzardPlates", reason: "kein BlizzardPlates.toc — WoW würde diesen Ordner nicht laden" },
  ],
  cache_hits: 1,
  hashed: 2,
};

function character(states: Record<string, boolean>): Character {
  return {
    account: "RYLON8",
    realm: "NostalGeek 1.12",
    name: "Zinnober",
    path: "/wtf/Zinnober/AddOns.txt",
    label: "Zinnober · NostalGeek 1.12 (RYLON8)",
    states,
  };
}

function mountTable(
  scan: AddonScan = SCAN,
  opts: {
    clientInterface?: string | null;
    character?: Character | null;
    /** Standard: wie der Client — veraltete werden nicht geladen. */
    loadsOutdated?: boolean;
  } = {},
) {
  return mount(AddonTable, {
    props: {
      scan,
      clientInterface: opts.clientInterface ?? null,
      character: opts.character ?? null,
      loadsOutdated: opts.loadsOutdated ?? true,
    },
  });
}

/** Titelspalte aller sichtbaren Datenzeilen, in Anzeigereihenfolge. */
function titles(wrapper: VueWrapper): string[] {
  return wrapper
    .findAll("tbody tr")
    .map((row) => row.findAll("td")[0]?.text() ?? "")
    .filter((text) => text.length > 0 && !text.includes("Ordner"))
    .map((text) => text.replace(/^[▸▼]\s*/, ""));
}

beforeEach(() => {
  setLocale("de");
});

describe("AddonTable – Darstellung", () => {
  it("listet alle Addons mit Kurz-Hash, Modus und Größe", () => {
    const wrapper = mountTable();
    const text = wrapper.text();
    expect(text).toContain("pfQuest");
    expect(text).toContain("7c1d90ffaa11");
    expect(text).toContain("74.4 MB");
    expect(wrapper.text()).toContain("3 von 3");
  });

  it("sortiert per Vorgabe alphabetisch nach Titel", () => {
    expect(titles(mountTable())).toEqual(["Accountant", "Invite-o-matik", "pfQuest"]);
  });

  it("zeigt einen Platzhalter statt einer leeren Zelle bei fehlender Version", () => {
    const wrapper = mountTable();
    const accountant = wrapper.findAll("tbody tr")[0];
    expect(accountant.findAll("td")[1].text()).toBe("—");
  });

  it("benennt nur Git-Checkouts, behauptet sonst nichts", () => {
    // „ZIP" wäre erfunden: wir sehen nur, ob ein .git/ da ist. Woher die
    // übrigen Dateien stammen, wissen wir nicht.
    const wrapper = mountTable();
    expect(wrapper.text()).toContain("Git");
    expect(wrapper.text()).not.toContain("ZIP");
    const modeCell = (row: number) => wrapper.findAll("tbody tr")[row].findAll("td")[4].text();
    expect(modeCell(0)).toBe("");
    expect(modeCell(1)).toBe("Git");
  });
});

describe("AddonTable – Sortierung", () => {
  it("kehrt die Reihenfolge beim Klick auf die Titelspalte um", async () => {
    const wrapper = mountTable();
    await wrapper.findAll("thead button")[0].trigger("click");
    expect(titles(wrapper)).toEqual(["pfQuest", "Invite-o-matik", "Accountant"]);
  });

  it("sortiert Zahlenspalten beim ersten Klick absteigend", async () => {
    // Bei „Dateien" und „Größe" ist „größte zuerst" die nützlichere Erwartung
    // als „14 Dateien zuerst" — TanStack macht das für Zahlen von sich aus.
    const wrapper = mountTable();
    await wrapper.findAll("thead button")[5].trigger("click");
    expect(titles(wrapper)).toEqual(["pfQuest", "Invite-o-matik", "Accountant"]);
    await wrapper.findAll("thead button")[5].trigger("click");
    expect(titles(wrapper)).toEqual(["Accountant", "Invite-o-matik", "pfQuest"]);
  });

  it("sortiert nach Version und Hash, mit Leerwert für fehlende Versionen", async () => {
    const wrapper = mountTable();
    await wrapper.findAll("thead button")[1].trigger("click");
    // Accountant hat keine Version und sortiert als Leerstring nach vorn.
    expect(titles(wrapper)).toEqual(["Accountant", "Invite-o-matik", "pfQuest"]);

    await wrapper.findAll("thead button")[3].trigger("click");
    expect(titles(wrapper)).toEqual(["Accountant", "pfQuest", "Invite-o-matik"]);
  });

  it("meldet die Sortierrichtung an Hilfstechnologie", async () => {
    const wrapper = mountTable();
    const header = () => wrapper.findAll("thead th")[0];
    expect(header().attributes("aria-sort")).toBe("ascending");
    await wrapper.findAll("thead button")[0].trigger("click");
    expect(header().attributes("aria-sort")).toBe("descending");
    expect(wrapper.findAll("thead th")[1].attributes("aria-sort")).toBe("none");
  });
});

describe("AddonTable – Filter", () => {
  it("filtert per Freitext über Titel und Autor", async () => {
    const wrapper = mountTable();
    // Accountant hat im Fixture keinen Autor und fällt daher heraus.
    await wrapper.find("input[type=search]").setValue("shagu");
    expect(titles(wrapper)).toEqual(["pfQuest"]);
    expect(wrapper.text()).toContain("1 von 3");
  });

  it("findet ein Addon über seinen Hash", async () => {
    const wrapper = mountTable();
    await wrapper.find("input[type=search]").setValue("a91f0c33");
    expect(titles(wrapper)).toEqual(["Invite-o-matik"]);
  });

  it("beschränkt auf Developer-Mode", async () => {
    const wrapper = mountTable();
    await wrapper.find("input[type=checkbox]").setValue(true);
    expect(titles(wrapper)).toEqual(["Invite-o-matik"]);
  });

  it("zeigt einen Hinweis, wenn nichts passt", async () => {
    const wrapper = mountTable();
    await wrapper.find("input[type=search]").setValue("existiert-nicht");
    expect(wrapper.text()).toContain("Keine Addons passen zur Suche.");
  });
});

describe("AddonTable – Detail-Ausklappung", () => {
  it("klappt Details erst auf Klick aus", async () => {
    const wrapper = mountTable();
    expect(wrapper.text()).not.toContain("A lightweight Questhelper");

    const toggle = wrapper.findAll("tbody tr")[2].find("td button");
    expect(toggle.attributes("aria-expanded")).toBe("false");
    await toggle.trigger("click");

    expect(wrapper.text()).toContain("A lightweight Questhelper");
    expect(wrapper.text()).toContain("Shagu");
    expect(wrapper.text()).toContain("/games/WoW/Interface/AddOns/pfQuest");
    // Voller Hash, nicht nur die Kurzform.
    expect(wrapper.text()).toContain(
      "7c1d90ffaa11223344556677889900aabbccddeeff00112233445566778899aa",
    );
  });

  it("klappt wieder zu", async () => {
    const wrapper = mountTable();
    const toggle = () => wrapper.findAll("tbody tr")[2].find("td button");
    await toggle().trigger("click");
    expect(toggle().attributes("aria-expanded")).toBe("true");
    await toggle().trigger("click");
    expect(wrapper.text()).not.toContain("A lightweight Questhelper");
  });

  it("markiert einen Hash aus dem Cache", async () => {
    const wrapper = mountTable();
    await wrapper.findAll("tbody tr")[1].find("td button").trigger("click");
    expect(wrapper.text()).toContain("aus Cache");
  });

  it("zeigt Platzhalter für fehlende Detailfelder", async () => {
    const wrapper = mountTable();
    await wrapper.findAll("tbody tr")[0].find("td button").trigger("click");
    const detail = wrapper.findAll("tbody tr")[1].text();
    expect(detail).toContain("Autor");
    expect(detail).toContain("—");
  });
});

describe("AddonTable – Titelfarben", () => {
  const coloured: AddonScan = {
    ...SCAN,
    addons: [
      addon({
        id: "ShaguPlates",
        title: "ShaguPlates",
        title_spans: [
          { text: "Shagu", color: "33ffcc" },
          { text: "Plates", color: "ffffff" },
        ],
      }),
    ],
  };

  /** Ersetzt `matchMedia`, das jsdom nicht mitbringt. */
  function stubMatchMedia(matches: boolean) {
    const listeners: Array<(event: MediaQueryListEvent) => void> = [];
    const mql = {
      matches,
      addEventListener: (_: string, fn: (event: MediaQueryListEvent) => void) =>
        listeners.push(fn),
      removeEventListener: vi.fn(),
    };
    vi.stubGlobal("matchMedia", () => mql);
    return {
      listeners,
      removeEventListener: mql.removeEventListener,
    };
  }

  it("färbt die Abschnitte des Autors ein", () => {
    const wrapper = mountTable(coloured);
    const spans = wrapper.findAll("tbody tr td span span");
    expect(spans.map((span) => span.text())).toEqual(["Shagu", "Plates"]);
    // Weiß auf Pergament wäre unsichtbar und wird nachgezogen.
    expect(spans[1].attributes("style")).not.toContain("255, 255, 255");
    expect(spans[1].attributes("style")).toContain("color");
  });

  it("färbt einen Ersatznamen nicht ein", () => {
    // Der Ordnername stammt nicht vom Autor — ihn zu färben wäre erfunden.
    const wrapper = mountTable({
      ...SCAN,
      addons: [addon({ id: "CT_BarMod", title: "CT_BarMod", title_spans: [] })],
    });
    expect(wrapper.findAll("tbody tr td span span")).toHaveLength(0);
    expect(wrapper.text()).toContain("CT_BarMod");
  });

  it("folgt einem Wechsel des Systemthemas", async () => {
    const { listeners, removeEventListener } = stubMatchMedia(false);
    const wrapper = mountTable(coloured);
    const light = wrapper.findAll("tbody tr td span span")[1].attributes("style");

    listeners.forEach((fn) => fn({ matches: true } as MediaQueryListEvent));
    await wrapper.vm.$nextTick();
    expect(wrapper.findAll("tbody tr td span span")[1].attributes("style")).not.toBe(light);

    // Beim Zerstören wieder abmelden, sonst hält der Listener die Komponente fest.
    wrapper.unmount();
    expect(removeEventListener).toHaveBeenCalled();
    vi.unstubAllGlobals();
  });
});

describe("AddonTable – Interface-Version", () => {
  it("markiert Addons, die der Client nicht ohne Weiteres lädt", () => {
    const scan: AddonScan = {
      ...SCAN,
      addons: [
        addon({ id: "Aktuell", title: "Aktuell", interface: "11200" }),
        addon({ id: "Alt", title: "Alt", interface: "11000" }),
        addon({ id: "Ohne", title: "Ohne", interface: null }),
      ],
    };
    const wrapper = mountTable(scan, { clientInterface: "11200" });
    const cell = (row: number) => wrapper.findAll("tbody tr")[row].findAll("td")[2];

    expect(cell(0).text()).toBe("11200");
    expect(cell(0).html()).not.toContain("⚠");
    expect(cell(1).text()).toContain("11000");
    expect(cell(1).html()).toContain("⚠");
    expect(cell(2).text()).toBe("—");
  });

  it("lässt sich nach Interface-Version sortieren", async () => {
    const scan: AddonScan = {
      ...SCAN,
      addons: [
        addon({ id: "Neu", title: "Neu", interface: "11200" }),
        addon({ id: "Alt", title: "Alt", interface: "11000" }),
        addon({ id: "Ohne", title: "Ohne", interface: null }),
      ],
    };
    const wrapper = mountTable(scan, { clientInterface: "11200" });
    await wrapper.findAll("thead button")[2].trigger("click");
    expect(titles(wrapper)).toEqual(["Ohne", "Alt", "Neu"]);
  });

  it("erklärt den Grund, statt nur zu markieren", () => {
    const scan: AddonScan = {
      ...SCAN,
      addons: [addon({ id: "Alt", title: "Alt", interface: "11000" })],
    };
    // Ohne diesen Hinweis sucht der Nutzer den Fehler beim Addon — oder
    // wundert sich, warum Einträge fehlen.
    const wrapper = mountTable(scan, { clientInterface: "11200", loadsOutdated: true });
    expect(wrapper.text()).toContain("Veraltete AddOns laden");
    expect(wrapper.text()).toContain("11200");
  });

  it("behauptet nichts, wenn die Client-Version unbekannt ist", () => {
    const wrapper = mountTable(SCAN, { clientInterface: null });
    expect(wrapper.html()).not.toContain("⚠");
    expect(wrapper.text()).not.toContain("Veraltete AddOns laden");
  });

  const mixed: AddonScan = {
    ...SCAN,
    addons: [
      addon({ id: "Aktuell", title: "Aktuell", interface: "11200" }),
      addon({ id: "Alt", title: "Alt", interface: "11000" }),
    ],
  };

  it("blendet veraltete aus, wenn der Client sie nicht lädt", () => {
    // Die Liste spiegelt, was im Spiel ankommt: was der Client nicht lädt,
    // existiert dort faktisch nicht.
    const wrapper = mountTable(mixed, { clientInterface: "11200", loadsOutdated: false });
    expect(titles(wrapper)).toEqual(["Aktuell"]);
    expect(wrapper.text()).toContain("ist hier ausgeblendet");
  });

  it("zeigt veraltete mit, wenn der Client sie lädt", () => {
    const wrapper = mountTable(mixed, { clientInterface: "11200", loadsOutdated: true });
    expect(titles(wrapper)).toEqual(["Aktuell", "Alt"]);
    expect(wrapper.text()).toContain("lädt es trotzdem");
  });

  it("holt ausgeblendete veraltete auf Wunsch zurück", async () => {
    const wrapper = mountTable(mixed, { clientInterface: "11200", loadsOutdated: false });
    const checkbox = wrapper.findAll("input[type=checkbox]")[1];
    expect((checkbox.element as HTMLInputElement).checked).toBe(false);
    await checkbox.setValue(true);
    expect(titles(wrapper)).toEqual(["Aktuell", "Alt"]);
  });

  it("lässt sie auf Wunsch auch wieder verschwinden", async () => {
    const wrapper = mountTable(mixed, { clientInterface: "11200", loadsOutdated: true });
    const checkbox = wrapper.findAll("input[type=checkbox]")[1];
    expect((checkbox.element as HTMLInputElement).checked).toBe(true);
    await checkbox.setValue(false);
    expect(titles(wrapper)).toEqual(["Aktuell"]);
  });
});

describe("AddonTable – Aktiv-Zustand", () => {
  const scan: AddonScan = {
    ...SCAN,
    addons: [
      addon({ id: "An", title: "An" }),
      addon({ id: "Aus", title: "Aus" }),
      addon({ id: "Ungesehen", title: "Ungesehen" }),
    ],
  };
  // Schlüssel kleingeschrieben, wie AddOns.txt sie liefert.
  const zinnober = character({ an: true, aus: false });

  it("zeigt ohne Charakter gar keine Aktiv-Spalte", () => {
    // In der reinen Addon-Ansicht gibt es keinen Charakter-Bezug — eine Spalte
    // voller Striche wäre nur Rauschen.
    const wrapper = mountTable(scan);
    const headers = wrapper.findAll("thead th").map((h) => h.text());
    expect(headers.some((h) => h.includes("Aktiv"))).toBe(false);
    expect(wrapper.findAll("tbody tr")[0].findAll("td")).toHaveLength(7);
  });

  it("unterscheidet mit Charakter aktiv, aus und nie gesehen", () => {
    const wrapper = mountTable(scan, { character: zinnober });
    const cell = (row: number) => wrapper.findAll("tbody tr")[row].findAll("td")[3].text();
    expect(cell(0)).toBe("aktiv");
    // „Nie gesehen" ist etwas anderes als „abgeschaltet".
    expect(cell(1)).toBe("—");
    expect(cell(2)).toBe("aus");
  });

  it("lässt Hash, Modus und Dateizahl weg — die zählen hier nicht", () => {
    const wrapper = mountTable(scan, { character: zinnober });
    const headers = wrapper.findAll("thead th").map((h) => h.text());
    expect(headers.some((h) => h.includes("Hash"))).toBe(false);
    expect(headers.some((h) => h.includes("Modus"))).toBe(false);
    expect(headers.some((h) => h.includes("Dateien"))).toBe(false);
    // Addon, Version, Interface, Aktiv, Größe
    expect(wrapper.findAll("tbody tr")[0].findAll("td")).toHaveLength(5);
  });

  it("hält das Weggelassene im Detailbereich bereit", async () => {
    const wrapper = mountTable(scan, { character: zinnober });
    await wrapper.findAll("tbody tr")[0].find("td button").trigger("click");
    const detail = wrapper.findAll("tbody tr")[1].text();
    expect(detail).toContain("Modus");
    expect(detail).toContain("Dateien");
    expect(detail).toContain("Tree-Hash");
  });

  it("sortiert beim Charakter von sich aus nach Aktiv, dann alphabetisch", async () => {
    // Die Frage der Ansicht lautet „was lädt dieser Charakter?" — das soll
    // ohne einen einzigen Klick beantwortet sein.
    const wrapper = mountTable(scan, { character: zinnober });
    expect(titles(wrapper)).toEqual(["An", "Ungesehen", "Aus"]);
    // Und „nie gesehen" liegt zwischen aktiv und aus, statt zu verschmelzen.
    await wrapper.findAll("thead button")[3].trigger("click");
    expect(titles(wrapper)).toEqual(["Aus", "Ungesehen", "An"]);
  });

  it("sortiert ohne Charakter weiterhin alphabetisch", () => {
    expect(titles(mountTable(scan))).toEqual(["An", "Aus", "Ungesehen"]);
  });

  it("zeigt DefaultState im Detail, getrennt vom echten Zustand", async () => {
    const wrapper = mountTable(scan, { character: zinnober });
    await wrapper.findAll("tbody tr")[0].find("td button").trigger("click");
    const detail = wrapper.findAll("tbody tr")[1].text();
    expect(detail).toContain("DefaultState");
    expect(detail).toContain("disabled");
  });
});

describe("AddonTable – Fehlerfälle", () => {
  const brokenScan: AddonScan = {
    ...SCAN,
    addons: [
      addon({
        id: "Kaputt",
        title: "Kaputt",
        tree_sha: null,
        tree_sha_short: null,
        error: "symlink is not hashable: /x/link.lua",
      }),
    ],
  };

  it("zeigt ein unhashbares Addon weiterhin an, mit Fehlermarkierung", async () => {
    // Ein Addon, das aus der Liste verschwindet, lässt den Nutzer an der
    // falschen Stelle suchen — dieselbe Begründung wie E-15 im Backend.
    const wrapper = mountTable(brokenScan);
    expect(wrapper.text()).toContain("Kaputt");
    expect(wrapper.text()).toContain("Fehler");
    expect(wrapper.findAll("tbody tr")[0].findAll("td")[3].text()).toBe("—");

    await wrapper.findAll("tbody tr")[0].find("td button").trigger("click");
    expect(wrapper.text()).toContain("symlink is not hashable");
  });

  it("bietet keinen Kopieren-Knopf ohne Hash", async () => {
    const wrapper = mountTable(brokenScan);
    await wrapper.findAll("tbody tr")[0].find("td button").trigger("click");
    expect(wrapper.text()).not.toContain("Kopieren");
  });
});

describe("AddonTable – Hash kopieren", () => {
  it("kopiert den vollen Hash und bestätigt", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", { ...navigator, clipboard: { writeText } });

    const wrapper = mountTable();
    await wrapper.findAll("tbody tr")[2].find("td button").trigger("click");
    const copyButton = wrapper
      .findAll("tbody tr")[3]
      .findAll("button")
      .find((b) => b.text() === "Kopieren")!;
    await copyButton.trigger("click");
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(writeText).toHaveBeenCalledWith(
      "7c1d90ffaa11223344556677889900aabbccddeeff00112233445566778899aa",
    );
    expect(wrapper.text()).toContain("Kopiert");
    vi.unstubAllGlobals();
  });

  it("bleibt still, wenn die Zwischenablage nicht verfügbar ist", async () => {
    // Unsichere Kontexte und Testumgebungen haben keine Clipboard-API — das
    // darf keinen Fehler auslösen.
    vi.stubGlobal("navigator", { ...navigator, clipboard: undefined });
    const wrapper = mountTable();
    await wrapper.findAll("tbody tr")[2].find("td button").trigger("click");
    const copyButton = wrapper
      .findAll("tbody tr")[3]
      .findAll("button")
      .find((b) => b.text() === "Kopieren")!;
    await copyButton.trigger("click");
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(wrapper.text()).not.toContain("Kopiert");
    vi.unstubAllGlobals();
  });

  it("meldet einen Fehlschlag der Zwischenablage nicht als Erfolg", async () => {
    const writeText = vi.fn().mockRejectedValue(new Error("verweigert"));
    vi.stubGlobal("navigator", { ...navigator, clipboard: { writeText } });
    const wrapper = mountTable();
    await wrapper.findAll("tbody tr")[2].find("td button").trigger("click");
    const copyButton = wrapper
      .findAll("tbody tr")[3]
      .findAll("button")
      .find((b) => b.text() === "Kopieren")!;
    await copyButton.trigger("click");
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(wrapper.text()).not.toContain("Kopiert");
    vi.unstubAllGlobals();
  });
});

describe("AddonTable – übersprungene Ordner", () => {
  it("nennt die Anzahl und klappt die Gründe auf Wunsch aus", async () => {
    const wrapper = mountTable();
    expect(wrapper.text()).toContain("1 Ordner übersprungen");
    expect(wrapper.text()).not.toContain("BlizzardPlates");

    const toggle = wrapper.findAll("button").find((b) => b.text().includes("übersprungen"))!;
    await toggle.trigger("click");
    expect(wrapper.text()).toContain("BlizzardPlates");
    expect(wrapper.text()).toContain("kein BlizzardPlates.toc");
  });

  it("beugt Singular und Plural korrekt", () => {
    setLocale("en");
    const one = mountTable();
    expect(one.text()).toContain("1 folder skipped");
    expect(one.text()).not.toContain("1 folders");

    const many = mountTable({
      ...SCAN,
      skipped: [
        { id: "A", reason: "kein A.toc" },
        { id: "B", reason: "kein B.toc" },
      ],
    });
    expect(many.text()).toContain("2 folders skipped");
  });

  it("erscheint nicht in der Charakter-Ansicht", () => {
    // Welcher Ordner kein gültiges .toc hat, ist eine Eigenschaft der
    // Installation, nicht eines Charakters.
    const wrapper = mountTable(SCAN, { character: character({}) });
    expect(wrapper.text()).not.toContain("übersprungen");
  });

  it("blendet den Abschnitt aus, wenn nichts übersprungen wurde", () => {
    const wrapper = mountTable({ ...SCAN, skipped: [] });
    expect(wrapper.text()).not.toContain("übersprungen");
  });
});
