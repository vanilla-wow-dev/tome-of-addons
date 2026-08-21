import { describe, it, expect, beforeEach } from "vitest";
import { mount, config } from "@vue/test-utils";
import { i18n, setLocale } from "./i18n";
import type { Addon, AddonScan, Character } from "./addons";
import CharacterView from "./CharacterView.vue";

config.global.plugins = [i18n];

function addon(overrides: Partial<Addon> = {}): Addon {
  return {
    id: "pfQuest",
    path: "/games/WoW/Interface/AddOns/pfQuest",
    title: "pfQuest",
    version: null,
    interface: "11200",
    notes: null,
    author: null,
    tree_sha: "7c1d90ffaa11" + "0".repeat(52),
    tree_sha_short: "7c1d90ffaa11",
    mode: "consumer",
    default_state: "disabled",
    file_count: 3,
    size_bytes: 1024,
    cached: false,
    error: null,
    ...overrides,
  };
}

function scanOf(addons: Addon[]): AddonScan {
  return { addons, skipped: [], cache_hits: 0, hashed: addons.length };
}

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

beforeEach(() => setLocale("de"));

describe("CharacterView", () => {
  const scan = scanOf([
    addon({ id: "An", title: "An" }),
    addon({ id: "Aus", title: "Aus" }),
    addon({ id: "Ungesehen", title: "Ungesehen" }),
    addon({ id: "AltUndAn", title: "AltUndAn", interface: "11000" }),
  ]);

  function mountView(clientInterface: string | null = "11200") {
    return mount(CharacterView, {
      props: {
        character: character({ an: true, aus: false, altundan: true }),
        scan,
        clientInterface,
        loadsOutdated: true,
      },
    });
  }

  it("nennt Name, Realm und Account in der Überschrift", () => {
    expect(mountView().text()).toContain("Zinnober · NostalGeek 1.12 (RYLON8)");
  });

  it("zählt aktiv, aus und nie gesehen getrennt", () => {
    // Die drei Zustände dürfen nicht verschmelzen — „nie gesehen" heißt, der
    // Client kennt das Addon noch gar nicht.
    expect(mountView().text()).toContain("2 aktiv · 1 aus · 1 nie gesehen");
  });

  it("warnt vor aktiven Addons, die der Client trotzdem nicht lädt", () => {
    // Der eigentlich verwirrende Fall: Häkchen gesetzt, lädt aber nicht.
    const text = mountView().text();
    expect(text).toContain("1 aktives Addon ist veraltet");
  });

  it("warnt nicht, wenn die Client-Version unbekannt ist", () => {
    expect(mountView(null).text()).not.toContain("veraltet und wird");
  });

  it("beugt die Warnung im Plural", () => {
    const many = mount(CharacterView, {
      props: {
        character: character({ a: true, b: true }),
        scan: scanOf([
          addon({ id: "A", title: "A", interface: "11000" }),
          addon({ id: "B", title: "B", interface: "11100" }),
        ]),
        clientInterface: "11200",
        loadsOutdated: true,
      },
    });
    expect(many.text()).toContain("2 aktive Addons sind veraltet");
  });

  it("reicht den Charakter an die Tabelle durch", () => {
    const wrapper = mountView();
    // Aktiv-Spalte ist vorhanden, weil ein Charakter gesetzt ist.
    // Absteigend vorsortiert: aktiv zuerst.
    expect(wrapper.findAll("thead th").map((h) => h.text())).toContain("Aktiv ▼");
    expect(wrapper.text()).toContain("aktiv");
  });
});
