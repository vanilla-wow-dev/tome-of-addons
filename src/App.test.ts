import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises, type VueWrapper } from "@vue/test-utils";
import type { WowExeInfo, WowRoot } from "./wow";

// Tauri-APIs mocken. vi.hoisted, damit die Mock-Fns vor den vi.mock-Factories existieren.
const { invoke, getVersion, check, relaunch } = vi.hoisted(() => ({
  invoke: vi.fn(),
  getVersion: vi.fn(),
  check: vi.fn(),
  relaunch: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check, Update: class {} }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch }));

import App from "./App.vue";

const ROOT: WowRoot = {
  path: "/games/WoW",
  has_exe: true,
  has_mpq: true,
  has_interface: true,
  has_addons: true,
  method: "walkup",
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

/** Standard-Mock-Verhalten: Version gesetzt, keine Roots, kein Update. */
function setDefaults() {
  getVersion.mockResolvedValue("0.1.0");
  check.mockResolvedValue(null);
  invoke.mockImplementation((cmd: string) => {
    if (cmd === "detect_wow_roots_command") return Promise.resolve([]);
    if (cmd === "inspect_wow_exe_command") return Promise.resolve(EXE);
    return Promise.resolve(null);
  });
}

/** Findet einen Button anhand seines Textinhalts. */
function buttonByText(wrapper: VueWrapper, text: string) {
  return wrapper.findAll("button").find((b) => b.text().includes(text))!;
}

beforeEach(() => {
  vi.clearAllMocks();
  setDefaults();
});

describe("App – Mount & Version", () => {
  it("zeigt die App-Version nach dem Mounten", async () => {
    getVersion.mockResolvedValue("1.2.3");
    const wrapper = mount(App);
    await flushPromises();
    expect(wrapper.find(".version").text()).toBe("v1.2.3");
  });

  it("löst die WoW-Erkennung beim Start aus", async () => {
    mount(App);
    await flushPromises();
    expect(invoke).toHaveBeenCalledWith("detect_wow_roots_command");
  });
});

describe("App – WoW-Erkennung", () => {
  it("rendert gefundene Roots inkl. Exe-Verdikt", async () => {
    invoke.mockImplementation((cmd: string) => {
      if (cmd === "detect_wow_roots_command") return Promise.resolve([ROOT]);
      if (cmd === "inspect_wow_exe_command") return Promise.resolve(EXE);
      return Promise.resolve(null);
    });
    const wrapper = mount(App);
    await flushPromises();

    expect(wrapper.text()).toContain("/games/WoW");
    expect(wrapper.text()).toContain("✓ Offiziell 1.12.1 (enUS)");
    expect(wrapper.text()).toContain("5875");
    expect(wrapper.text()).toContain(EXE.sha1);
  });

  it("zeigt eine Meldung, wenn nichts gefunden wird", async () => {
    const wrapper = mount(App);
    await flushPromises();
    expect(wrapper.text()).toContain("Keine WoW-1.12.1-Installation gefunden");
  });

  it("zeigt einen Fehler, wenn die Erkennung wirft", async () => {
    invoke.mockImplementation((cmd: string) => {
      if (cmd === "detect_wow_roots_command") return Promise.reject(new Error("boom"));
      return Promise.resolve(null);
    });
    const wrapper = mount(App);
    await flushPromises();
    expect(wrapper.text()).toContain("WoW-Suche fehlgeschlagen");
  });

  it("rendert den Root auch wenn die Exe-Analyse fehlschlägt", async () => {
    invoke.mockImplementation((cmd: string) => {
      if (cmd === "detect_wow_roots_command") return Promise.resolve([ROOT]);
      if (cmd === "inspect_wow_exe_command") return Promise.reject(new Error("unlesbar"));
      return Promise.resolve(null);
    });
    const wrapper = mount(App);
    await flushPromises();
    expect(wrapper.text()).toContain("/games/WoW");
    // Kein Exe-Detailblock, aber kein Crash.
    expect(wrapper.find(".exe").exists()).toBe(false);
  });

  it("rendert einen Root ohne AddOns-Ordner und ohne Build-Datum", async () => {
    const root = { ...ROOT, has_addons: false };
    const exe = { ...EXE, build_date: null, identity: { status: "unknown-build" } as const };
    invoke.mockImplementation((cmd: string) => {
      if (cmd === "detect_wow_roots_command") return Promise.resolve([root]);
      if (cmd === "inspect_wow_exe_command") return Promise.resolve(exe);
      return Promise.resolve(null);
    });
    const wrapper = mount(App);
    await flushPromises();
    expect(wrapper.text()).toContain("Unbekannter Build");
  });
});

describe("App – Update-Flow", () => {
  it("meldet ein verfügbares Update", async () => {
    check.mockResolvedValue({ version: "0.2.0", downloadAndInstall: vi.fn() });
    const wrapper = mount(App);
    await flushPromises();

    await buttonByText(wrapper, "Auf Updates prüfen").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("0.2.0");
    expect(buttonByText(wrapper, "herunterladen")).toBeTruthy();
  });

  it("meldet, wenn aktuell", async () => {
    check.mockResolvedValue(null);
    const wrapper = mount(App);
    await flushPromises();

    await buttonByText(wrapper, "Auf Updates prüfen").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("neuesten Stand");
  });

  it("meldet einen fehlgeschlagenen Update-Check", async () => {
    check.mockRejectedValue(new Error("kein Netz"));
    const wrapper = mount(App);
    await flushPromises();

    await buttonByText(wrapper, "Auf Updates prüfen").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("Update-Check fehlgeschlagen");
  });

  it("führt Download + Restart-Flow durch", async () => {
    const downloadAndInstall = vi.fn(async (cb: (e: any) => void) => {
      cb({ event: "Started", data: { contentLength: 100 } });
      cb({ event: "Progress", data: { chunkLength: 100 } });
      cb({ event: "Finished" });
    });
    check.mockResolvedValue({ version: "0.2.0", downloadAndInstall });
    const wrapper = mount(App);
    await flushPromises();

    await buttonByText(wrapper, "Auf Updates prüfen").trigger("click");
    await flushPromises();
    await buttonByText(wrapper, "herunterladen").trigger("click");
    await flushPromises();

    expect(downloadAndInstall).toHaveBeenCalled();
    expect(wrapper.text()).toContain("Neustart erforderlich");

    await buttonByText(wrapper, "neustarten").trigger("click");
    expect(relaunch).toHaveBeenCalled();
  });

  it("zeigt Download-Fortschritt ohne bekannte Gesamtgröße", async () => {
    let emit: ((e: any) => void) | null = null;
    const downloadAndInstall = vi.fn((cb: (e: any) => void) => {
      emit = cb;
      return new Promise<void>(() => {}); // bleibt im Download-Status hängen
    });
    check.mockResolvedValue({ version: "0.2.0", downloadAndInstall });
    const wrapper = mount(App);
    await flushPromises();
    await buttonByText(wrapper, "Auf Updates prüfen").trigger("click");
    await flushPromises();
    await buttonByText(wrapper, "herunterladen").trigger("click");
    await flushPromises();

    // Started ohne contentLength → total bleibt null (Zweig `?? null`).
    emit!({ event: "Started", data: {} });
    emit!({ event: "Progress", data: { chunkLength: 50 } });
    await flushPromises();
    expect(wrapper.text()).toContain("Lade…");
    expect(wrapper.text()).not.toContain(" / "); // keine "x / y"-Anzeige ohne total
  });

  it("meldet einen fehlgeschlagenen Download", async () => {
    const downloadAndInstall = vi.fn().mockRejectedValue(new Error("Plattenfehler"));
    check.mockResolvedValue({ version: "0.2.0", downloadAndInstall });
    const wrapper = mount(App);
    await flushPromises();
    await buttonByText(wrapper, "Auf Updates prüfen").trigger("click");
    await flushPromises();
    await buttonByText(wrapper, "herunterladen").trigger("click");
    await flushPromises();
    expect(wrapper.text()).toContain("Download fehlgeschlagen");
  });
});
