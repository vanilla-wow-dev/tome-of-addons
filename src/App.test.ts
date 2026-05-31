import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises, config, type VueWrapper } from "@vue/test-utils";
import type { Detection, WowExeInfo, WowRoot } from "./wow";
import { i18n, setLocale } from "./i18n";

// i18n-Plugin global bereitstellen; Tests laufen auf Deutsch.
config.global.plugins = [i18n];

// Tauri-APIs mocken. vi.hoisted, damit die Mock-Fns vor den vi.mock-Factories existieren.
const { invoke, getVersion, check, relaunch, exit } = vi.hoisted(() => ({
  invoke: vi.fn(),
  getVersion: vi.fn(),
  check: vi.fn(),
  relaunch: vi.fn(),
  exit: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check, Update: class {} }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch, exit }));

import App from "./App.vue";

const ROOT: WowRoot = {
  path: "/games/WoW",
  has_exe: true,
  has_mpq: true,
  has_interface: true,
  has_addons: true,
  method: "walkup",
};

const OTHER: WowRoot = {
  path: "/other/WoW",
  has_exe: true,
  has_mpq: true,
  has_interface: true,
  has_addons: false,
  method: "registry",
};

const EXE: WowExeInfo = {
  path: "/games/WoW/WoW.exe",
  size_bytes: 4775986,
  build: 5875,
  build_date: "Sep 19 2006 20:32:39",
  sha1: "893def24f703fd18c1514d31b92f00e616d8375f",
  md5: "ccf83146dbb3d10ef826aa4de178a5be",
  identity: { status: "official", version: "1.12.1", locale: "enUS" },
};

/** invoke-Mock, das per Command-Namen verzweigt. */
function mockInvoke(detection: Detection, opts: { exeFails?: boolean; relocateFails?: boolean } = {}) {
  invoke.mockImplementation((cmd: string) => {
    if (cmd === "detect_command") return Promise.resolve(detection);
    if (cmd === "inspect_wow_exe_command")
      return opts.exeFails ? Promise.reject(new Error("unlesbar")) : Promise.resolve(EXE);
    if (cmd === "relocate_into_command")
      return opts.relocateFails
        ? Promise.reject(new Error("Plattenfehler"))
        : Promise.resolve("/other/WoW/tome-of-addons");
    return Promise.resolve(null);
  });
}

function buttonByText(wrapper: VueWrapper, text: string) {
  return wrapper.findAll("button").find((b) => b.text().includes(text))!;
}

beforeEach(() => {
  vi.clearAllMocks();
  setLocale("de");
  getVersion.mockResolvedValue("0.1.0");
  check.mockResolvedValue(null);
  exit.mockResolvedValue(undefined);
  mockInvoke({ managed: null, suggestions: [] });
});

describe("App – Mount & Version", () => {
  it("zeigt die App-Version nach dem Mounten", async () => {
    getVersion.mockResolvedValue("1.2.3");
    const wrapper = mount(App);
    await flushPromises();
    expect(wrapper.find(".version").text()).toBe("v1.2.3");
  });

  it("löst die Erkennung beim Start aus", async () => {
    mount(App);
    await flushPromises();
    expect(invoke).toHaveBeenCalledWith("detect_command");
  });
});

describe("App – Verankert (managed)", () => {
  it("zeigt die verwaltete Installation inkl. Exe-Verdikt", async () => {
    mockInvoke({ managed: ROOT, suggestions: [] });
    const wrapper = mount(App);
    await flushPromises();

    expect(wrapper.text()).toContain("Verwaltet");
    expect(wrapper.text()).toContain("/games/WoW");
    expect(wrapper.text()).toContain("✓ Offiziell 1.12.1 (enUS)");
  });

  it("listet weitere Funde als „nicht verwaltet“ und kann dorthin verschieben", async () => {
    mockInvoke({ managed: ROOT, suggestions: [OTHER] });
    const wrapper = mount(App);
    await flushPromises();

    expect(wrapper.text()).toContain("nicht verwaltet");
    expect(wrapper.text()).toContain("/other/WoW");

    await buttonByText(wrapper, "hierher verschieben").trigger("click");
    await flushPromises();
    expect(invoke).toHaveBeenCalledWith("relocate_into_command", { targetRoot: "/other/WoW" });
  });

  it("rendert den Root auch wenn die Exe-Analyse fehlschlägt", async () => {
    mockInvoke({ managed: ROOT, suggestions: [] }, { exeFails: true });
    const wrapper = mount(App);
    await flushPromises();
    expect(wrapper.text()).toContain("/games/WoW");
    expect(wrapper.find(".exe").exists()).toBe(false);
  });
});

describe("App – Nicht verankert", () => {
  it("warnt und bietet die erkannten Installationen zum Verschieben an", async () => {
    mockInvoke({ managed: null, suggestions: [OTHER] });
    const wrapper = mount(App);
    await flushPromises();

    expect(wrapper.text()).toContain("nicht in einem WoW-Ordner");
    expect(wrapper.text()).toContain("/other/WoW");
    expect(buttonByText(wrapper, "hierher verschieben")).toBeTruthy();
  });

  it("zeigt einen Hinweis, wenn gar nichts gefunden wird", async () => {
    mockInvoke({ managed: null, suggestions: [] });
    const wrapper = mount(App);
    await flushPromises();
    expect(wrapper.text()).toContain("Keine WoW-1.12.1-Installation gefunden");
  });

  it("wechselt die UI-Sprache über den Umschalter", async () => {
    mockInvoke({ managed: null, suggestions: [] });
    const wrapper = mount(App);
    await flushPromises();
    expect(wrapper.text()).toContain("Keine WoW-1.12.1-Installation gefunden");

    await wrapper.find("select[aria-label='language']").setValue("en");
    expect(wrapper.text()).toContain("No WoW 1.12.1 installation found");
  });

  it("zeigt einen Fehler, wenn die Erkennung wirft", async () => {
    invoke.mockImplementation((cmd: string) =>
      cmd === "detect_command" ? Promise.reject(new Error("boom")) : Promise.resolve(null),
    );
    const wrapper = mount(App);
    await flushPromises();
    expect(wrapper.text()).toContain("WoW-Suche fehlgeschlagen");
  });
});

describe("App – Verschieben-Aktion", () => {
  it("kopiert in den Zielordner und schließt die Instanz", async () => {
    mockInvoke({ managed: null, suggestions: [OTHER] });
    const wrapper = mount(App);
    await flushPromises();

    await buttonByText(wrapper, "hierher verschieben").trigger("click");
    await flushPromises();

    expect(invoke).toHaveBeenCalledWith("relocate_into_command", { targetRoot: "/other/WoW" });
    expect(exit).toHaveBeenCalledWith(0);
    expect(wrapper.text()).toContain("kopiert");
  });

  it("meldet einen Fehlschlag beim Verschieben", async () => {
    mockInvoke({ managed: null, suggestions: [OTHER] }, { relocateFails: true });
    const wrapper = mount(App);
    await flushPromises();

    await buttonByText(wrapper, "hierher verschieben").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("Verschieben fehlgeschlagen");
    expect(exit).not.toHaveBeenCalled();
  });
});

describe("App – Update-Flow (automatischer Check)", () => {
  it("prüft automatisch beim Start und zeigt den Banner bei verfügbarem Update", async () => {
    check.mockResolvedValue({ version: "0.2.0", downloadAndInstall: vi.fn() });
    const wrapper = mount(App);
    await flushPromises();

    expect(check).toHaveBeenCalled();
    expect(wrapper.text()).toContain("0.2.0");
    expect(buttonByText(wrapper, "herunterladen")).toBeTruthy();
  });

  it("bleibt still, wenn kein Update vorliegt", async () => {
    check.mockResolvedValue(null);
    const wrapper = mount(App);
    await flushPromises();

    expect(check).toHaveBeenCalled();
    expect(wrapper.find(".banner").exists()).toBe(false);
    expect(wrapper.text()).not.toContain("0.2.0");
  });

  it("bleibt still, wenn der Check fehlschlägt", async () => {
    check.mockRejectedValue(new Error("kein Netz"));
    const wrapper = mount(App);
    await flushPromises();

    expect(wrapper.find(".banner").exists()).toBe(false);
    // Kein sichtbarer Fehler-Hinweis bei Hintergrund-Check.
    expect(wrapper.text()).not.toContain("fehlgeschlagen");
  });

  it("räumt das Check-Intervall beim Unmount auf", async () => {
    const wrapper = mount(App);
    await flushPromises();
    expect(() => wrapper.unmount()).not.toThrow();
  });

  it("lädt auf Klick herunter und führt den Restart-Flow durch", async () => {
    const downloadAndInstall = vi.fn(async (cb: (e: any) => void) => {
      cb({ event: "Started", data: { contentLength: 100 } });
      cb({ event: "Progress", data: { chunkLength: 100 } });
      cb({ event: "Finished" });
    });
    check.mockResolvedValue({ version: "0.2.0", downloadAndInstall });
    const wrapper = mount(App);
    await flushPromises();

    await buttonByText(wrapper, "herunterladen").trigger("click");
    await flushPromises();

    expect(downloadAndInstall).toHaveBeenCalled();
    expect(wrapper.text()).toContain("Neustart erforderlich");

    await buttonByText(wrapper, "neustarten").trigger("click");
    expect(relaunch).toHaveBeenCalled();
  });

  it("funktioniert mit einer Klasseninstanz mit privatem Feld (Regression #rid)", async () => {
    // Reproduziert den Reaktivitäts-Bug: eine echte Klasse mit privatem Feld
    // bricht, wenn sie in einem reaktiven ref (Proxy) statt shallowRef liegt.
    class FakeUpdate {
      #version: string;
      version: string;
      constructor(v: string) {
        this.#version = v;
        this.version = v;
      }
      async downloadAndInstall(cb: (e: any) => void) {
        void this.#version; // wirft bei Proxy-`this`
        cb({ event: "Started", data: { contentLength: 1 } });
        cb({ event: "Finished" });
      }
    }
    check.mockResolvedValue(new FakeUpdate("0.2.0"));
    const wrapper = mount(App);
    await flushPromises();

    await buttonByText(wrapper, "herunterladen").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("Neustart erforderlich");
    expect(wrapper.text()).not.toContain("Download fehlgeschlagen");
  });

  it("zeigt Download-Fortschritt ohne bekannte Gesamtgröße", async () => {
    let emit: ((e: any) => void) | null = null;
    const downloadAndInstall = vi.fn((cb: (e: any) => void) => {
      emit = cb;
      return new Promise<void>(() => {});
    });
    check.mockResolvedValue({ version: "0.2.0", downloadAndInstall });
    const wrapper = mount(App);
    await flushPromises();
    await buttonByText(wrapper, "herunterladen").trigger("click");
    await flushPromises();

    emit!({ event: "Started", data: {} });
    emit!({ event: "Progress", data: { chunkLength: 50 } });
    await flushPromises();
    expect(wrapper.text()).toContain("Lade…");
    expect(wrapper.text()).not.toContain(" / ");
  });

  it("meldet einen fehlgeschlagenen Download", async () => {
    const downloadAndInstall = vi.fn().mockRejectedValue(new Error("Plattenfehler"));
    check.mockResolvedValue({ version: "0.2.0", downloadAndInstall });
    const wrapper = mount(App);
    await flushPromises();
    await buttonByText(wrapper, "herunterladen").trigger("click");
    await flushPromises();
    expect(wrapper.text()).toContain("Download fehlgeschlagen");
  });
});
