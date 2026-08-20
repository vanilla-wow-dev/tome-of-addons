import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import RootCard from "./RootCard.vue";
import { i18n, setLocale } from "./i18n";
import type { WowExeInfo, WowRoot } from "./wow";

const ROOT: WowRoot = {
  path: "/W",
  has_exe: true,
  has_mpq: true,
  has_interface: true,
  has_addons: true,
  method: "walkup",
};

function exe(identity: WowExeInfo["identity"], extra: Partial<WowExeInfo> = {}): WowExeInfo {
  return {
    path: "/W/WoW.exe",
    size_bytes: 100,
    build: 5875,
    build_date: "Sep 19 2006",
    sha1: "a",
    md5: "b",
    identity,
    interface_version: "11200",
    ...extra,
  };
}

const mountCard = (props: { root: WowRoot; exe?: WowExeInfo | null }) =>
  mount(RootCard, { props, global: { plugins: [i18n] } });

beforeEach(() => setLocale("de"));

describe("RootCard", () => {
  it("zeigt ohne exe keinen Detailblock", () => {
    // Am Inhalt geprüft, nicht an einer CSS-Klasse: die überlebt keine
    // Umgestaltung und der Test würde lautlos wirkungslos.
    const w = mountCard({ root: ROOT });
    expect(w.text()).toContain("/W");
    expect(w.text()).not.toContain("SHA-1");
    expect(w.text()).not.toContain("MD5");
    expect(w.findAll("dl")).toHaveLength(0);
  });

  it("zeigt die Marker der Installation", () => {
    const w = mountCard({ root: { ...ROOT, has_addons: false } });
    expect(w.text()).toContain("WoW.exe");
    expect(w.text()).toContain("MPQ");
    expect(w.text()).toContain("AddOns");
  });

  it("identität: offiziell", () => {
    const w = mountCard({ root: ROOT, exe: exe({ status: "official", version: "1.12.1", locale: "enUS" }) });
    expect(w.text()).toContain("Offiziell 1.12.1 (enUS)");
  });

  it("identität: modifiziert", () => {
    const w = mountCard({ root: ROOT, exe: exe({ status: "modified", claims_version: "1.12.1" }) });
    expect(w.text()).toContain("Modifiziert");
    expect(w.text()).toContain("1.12.1");
  });

  it("identität: unbekannter Build, ohne Build-Datum und ohne Build-Nummer", () => {
    const w = mountCard({
      root: { ...ROOT, has_addons: false },
      exe: exe({ status: "unknown-build" }, { build: null, build_date: null }),
    });
    expect(w.text()).toContain("Unbekannter Build");
    expect(w.text()).toContain("—");
  });

  it("identität: kein erkennbarer Client", () => {
    const w = mountCard({ root: ROOT, exe: exe({ status: "unknown" }) });
    expect(w.text()).toContain("Kein erkennbarer");
  });

  it("übersetzt das Verdikt mit der aktiven Sprache", () => {
    setLocale("en");
    const w = mountCard({ root: ROOT, exe: exe({ status: "unknown" }) });
    expect(w.text()).toContain("Not a recognizable WoW client");
  });
});
