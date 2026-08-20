import { describe, it, expect, beforeEach, vi } from "vitest";
import { mount, config, type VueWrapper } from "@vue/test-utils";
import { i18n, setLocale } from "./i18n";
import type { Addon, AddonScan } from "./addons";
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
    file_count: 2310,
    size_bytes: 78_000_000,
    cached: false,
    error: null,
    ...overrides,
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

function mountTable(scan: AddonScan = SCAN) {
  return mount(AddonTable, { props: { scan } });
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

  it("hebt Developer-Mode vom Consumer-Mode ab", () => {
    const wrapper = mountTable();
    expect(wrapper.text()).toContain("Git");
    expect(wrapper.text()).toContain("ZIP");
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
    await wrapper.findAll("thead button")[4].trigger("click");
    expect(titles(wrapper)).toEqual(["pfQuest", "Invite-o-matik", "Accountant"]);
    await wrapper.findAll("thead button")[4].trigger("click");
    expect(titles(wrapper)).toEqual(["Accountant", "Invite-o-matik", "pfQuest"]);
  });

  it("sortiert nach Version und Hash, mit Leerwert für fehlende Versionen", async () => {
    const wrapper = mountTable();
    await wrapper.findAll("thead button")[1].trigger("click");
    // Accountant hat keine Version und sortiert als Leerstring nach vorn.
    expect(titles(wrapper)).toEqual(["Accountant", "Invite-o-matik", "pfQuest"]);

    await wrapper.findAll("thead button")[2].trigger("click");
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
    expect(wrapper.findAll("tbody tr")[0].findAll("td")[2].text()).toBe("—");

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

  it("blendet den Abschnitt aus, wenn nichts übersprungen wurde", () => {
    const wrapper = mountTable({ ...SCAN, skipped: [] });
    expect(wrapper.text()).not.toContain("übersprungen");
  });
});
