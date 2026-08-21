import { describe, it, expect, beforeEach } from "vitest";
import { mount, config } from "@vue/test-utils";
import { i18n, setLocale } from "./i18n";
import type { Addon, Character, HealthVerdict } from "./addons";
import SideBar from "./SideBar.vue";

config.global.plugins = [i18n];

function addon(id: string): Addon {
  return {
    id,
    path: `/games/WoW/Interface/AddOns/${id}`,
    title: id,
    title_spans: [{ text: id, color: null }],
    title_raw: id,
    version: null,
    interface: "11200",
    notes: null,
    author: null,
    tree_sha: null,
    tree_sha_short: null,
    mode: "consumer",
    default_state: null,
    file_count: 1,
    size_bytes: 1,
    cached: false,
    error: null,
  };
}

const CHARACTERS: Character[] = [
  {
    account: "RYLON8",
    realm: "NostalGeek 1.12",
    name: "Zinnober",
    path: "/wtf/z",
    label: "Zinnober · NostalGeek 1.12 (RYLON8)",
    states: { pfquest: true, pfui: true, geloescht: true },
  },
  {
    account: "RYLON14",
    realm: "NostalGeek 1.12",
    name: "Haensel",
    path: "/wtf/h",
    label: "Haensel · NostalGeek 1.12 (RYLON14)",
    states: { pfquest: false },
  },
];

function mountBar(
  opts: { health?: HealthVerdict; characters?: Character[]; view?: string } = {},
) {
  return mount(SideBar, {
    props: {
      health: opts.health ?? { level: "ok", reasons: [] },
      addons: [addon("pfQuest"), addon("pfUI")],
      characters: opts.characters ?? CHARACTERS,
      view: opts.view ?? "wow",
    },
  });
}

beforeEach(() => setLocale("de"));

describe("SideBar", () => {
  it("zeigt Addon-Anzahl und aktive Addons je Charakter", () => {
    const wrapper = mountBar();
    const entries = wrapper.findAll("button").map((b) => b.text());
    expect(entries[1]).toContain("2"); // zwei installierte Addons
    // „geloescht" steht in AddOns.txt, ist aber nicht installiert und zählt nicht.
    expect(entries[2]).toContain("Zinnober");
    expect(entries[2]).toContain("2");
    expect(entries[3]).toContain("Haensel");
    expect(entries[3]).toContain("0");
  });

  it("meldet jeden Ansichtswechsel, auch den Rückweg zur Installation", async () => {
    const wrapper = mountBar({ view: "addons" });
    const buttons = wrapper.findAll("button");
    await buttons[1].trigger("click");
    await buttons[2].trigger("click");
    await buttons[0].trigger("click");
    expect(wrapper.emitted("select")).toEqual([["addons"], ["/wtf/z"], ["wow"]]);
  });

  it("markiert die aktive Ansicht für Hilfstechnologie", () => {
    const wrapper = mountBar({ view: "/wtf/z" });
    const current = wrapper.findAll("button").filter((b) => b.attributes("aria-current") === "page");
    expect(current).toHaveLength(1);
    expect(current[0].text()).toContain("Zinnober");
  });

  it("schreibt den Installations-Zustand aus, statt nur zu färben", () => {
    // Farbe allein ist für Farbfehlsichtige keine Information.
    expect(mountBar().text()).toContain("Alles in Ordnung");
    expect(mountBar({ health: { level: "warn", reasons: ["x"] } }).text()).toContain(
      "Mit Einschränkungen",
    );
    expect(mountBar({ health: { level: "error", reasons: ["x"] } }).text()).toContain(
      "Keine Installation",
    );
  });

  it("blendet die Charakter-Gruppe ohne Charaktere aus", () => {
    const wrapper = mountBar({ characters: [] });
    expect(wrapper.text()).not.toContain("Charaktere");
    expect(wrapper.findAll("button")).toHaveLength(2);
  });
});
