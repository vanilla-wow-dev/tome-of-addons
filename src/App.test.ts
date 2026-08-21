import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises, config, type VueWrapper } from "@vue/test-utils";
import type { Detection, WowExeInfo, WowRoot } from "./wow";
import { i18n, setLocale } from "./i18n";

// i18n-Plugin global bereitstellen; Tests laufen auf Deutsch.
config.global.plugins = [i18n];

// Tauri-APIs mocken. vi.hoisted, damit die Mock-Fns vor den vi.mock-Factories existieren.
const { invoke, getVersion, check, relaunch, exit, listen, unlisten } = vi.hoisted(() => {
  const unlisten = vi.fn();
  return {
    invoke: vi.fn(),
    getVersion: vi.fn(),
    check: vi.fn(),
    relaunch: vi.fn(),
    exit: vi.fn(),
    listen: vi.fn(),
    unlisten,
  };
});

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));
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
  interface_version: "11200",
};

const SCAN = {
  addons: [
    {
      id: "pfQuest",
      path: "/games/WoW/Interface/AddOns/pfQuest",
      title: "pfQuest",
      title_spans: [{ text: "pfQuest", color: null }],
      version: "GIT",
      interface: "11200",
      notes: null,
      author: "Shagu",
      tree_sha: "7c1d90ffaa11" + "0".repeat(52),
      tree_sha_short: "7c1d90ffaa11",
      mode: "consumer" as const,
      default_state: "disabled",
      file_count: 2310,
      size_bytes: 78_000_000,
      cached: false,
      error: null,
    },
  ],
  skipped: [],
  cache_hits: 0,
  hashed: 1,
};

/** invoke-Mock, das per Command-Namen verzweigt. */
function mockInvoke(
  detection: Detection,
  opts: {
    exeFails?: boolean;
    relocateFails?: boolean;
    scanFails?: boolean;
    scan?: typeof SCAN;
    characters?: Array<{
      account: string;
      realm: string;
      name: string;
      path: string;
      label: string;
      states: Record<string, boolean>;
    }>;
    loadsOutdated?: boolean;
  } = {},
) {
  invoke.mockImplementation((cmd: string) => {
    if (cmd === "detect_command") return Promise.resolve(detection);
    if (cmd === "inspect_wow_exe_command")
      return opts.exeFails ? Promise.reject(new Error("unlesbar")) : Promise.resolve(EXE);
    if (cmd === "relocate_into_command")
      return opts.relocateFails
        ? Promise.reject(new Error("Plattenfehler"))
        : Promise.resolve("/other/WoW/tome-of-addons");
    if (cmd === "client_state_command") return Promise.resolve("not-running");
    if (cmd === "wow_settings_command")
      return Promise.resolve({ loads_outdated_addons: opts.loadsOutdated ?? true });
    if (cmd === "list_characters_command") return Promise.resolve(opts.characters ?? []);
    if (cmd === "scan_addons_command")
      return opts.scanFails
        ? Promise.reject(new Error("Kein Interface/AddOns"))
        : Promise.resolve(opts.scan ?? null);
    return Promise.resolve(null);
  });
}

function buttonByText(wrapper: VueWrapper, text: string) {
  return wrapper.findAll("button").find((b) => b.text().includes(text))!;
}

/** Wechselt über die Seitenleiste in eine Ansicht. */
async function goTo(wrapper: VueWrapper, label: string) {
  await buttonByText(wrapper, label).trigger("click");
  await flushPromises();
}

beforeEach(() => {
  vi.clearAllMocks();
  setLocale("de");
  getVersion.mockResolvedValue("0.1.0");
  check.mockResolvedValue(null);
  exit.mockResolvedValue(undefined);
  listen.mockResolvedValue(unlisten);
  mockInvoke({ managed: null, suggestions: [] });
});

describe("App – Mount & Version", () => {
  it("zeigt die App-Version nach dem Mounten", async () => {
    getVersion.mockResolvedValue("1.2.3");
    const wrapper = mount(App);
    await flushPromises();
    expect(wrapper.find("[data-testid=version]").text()).toBe("v1.2.3");
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
    // Der Detailbereich bleibt leer — geprüft am Inhalt, nicht an einer
    // CSS-Klasse, die eine Umgestaltung lautlos wegnehmen könnte.
    expect(wrapper.text()).not.toContain("SHA-1");
    expect(wrapper.text()).not.toContain("Offiziell");
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

describe("App – Seitenleiste, Charaktere und Fortschritt", () => {
  const CHARACTER = {
    account: "RYLON8",
    realm: "NostalGeek 1.12",
    name: "Zinnober",
    path: "/wtf/Zinnober/AddOns.txt",
    label: "Zinnober · NostalGeek 1.12 (RYLON8)",
    states: { pfquest: true },
  };

  it("startet auf der WoW-Ansicht und wechselt über die Seitenleiste", async () => {
    mockInvoke({ managed: ROOT, suggestions: [] }, { scan: SCAN });
    const wrapper = mount(App);
    await flushPromises();

    // Anfangs die Installation, nicht die Addon-Liste.
    expect(wrapper.text()).toContain("Verwaltet");
    expect(wrapper.text()).not.toContain("7c1d90ffaa11");

    await goTo(wrapper, "Addons");
    expect(wrapper.text()).toContain("7c1d90ffaa11");
    expect(wrapper.text()).not.toContain("Verwaltet");
  });

  it("zeigt Charaktere als eigene Gruppe mit Anzahl aktiver Addons", async () => {
    mockInvoke({ managed: ROOT, suggestions: [] }, { scan: SCAN, characters: [CHARACTER] });
    const wrapper = mount(App);
    await flushPromises();

    expect(invoke).toHaveBeenCalledWith("list_characters_command", { root: "/games/WoW" });
    const entry = buttonByText(wrapper, "Zinnober");
    // pfQuest ist im Scan und für den Charakter aktiv.
    expect(entry.text()).toContain("1");

    await goTo(wrapper, "Zinnober");
    expect(wrapper.text()).toContain("Zinnober · NostalGeek 1.12 (RYLON8)");
    expect(wrapper.text()).toContain("1 aktiv");
  });

  it("wechselt den Charakter ohne erneuten Scan", async () => {
    // Die Zustände kommen mit der Charakterliste — ein Wechsel ist reine
    // Anzeige und darf keinen Rescan auslösen.
    mockInvoke({ managed: ROOT, suggestions: [] }, { scan: SCAN, characters: [CHARACTER] });
    const wrapper = mount(App);
    await flushPromises();
    const scansBefore = invoke.mock.calls.filter((c) => c[0] === "scan_addons_command").length;

    await goTo(wrapper, "Zinnober");
    const scansAfter = invoke.mock.calls.filter((c) => c[0] === "scan_addons_command").length;
    expect(scansAfter).toBe(scansBefore);
  });

  it("kommt ohne Charakterliste aus", async () => {
    invoke.mockImplementation((cmd: string) => {
      if (cmd === "detect_command") return Promise.resolve({ managed: ROOT, suggestions: [] });
      if (cmd === "inspect_wow_exe_command") return Promise.resolve(EXE);
      if (cmd === "client_state_command") return Promise.resolve("not-running");
      if (cmd === "wow_settings_command") return Promise.resolve({ loads_outdated_addons: true });
      if (cmd === "list_characters_command") return Promise.reject(new Error("kein WTF"));
      if (cmd === "scan_addons_command") return Promise.resolve(SCAN);
      return Promise.resolve(null);
    });
    const wrapper = mount(App);
    await flushPromises();
    await goTo(wrapper, "Addons");
    expect(wrapper.text()).toContain("pfQuest");
    expect(wrapper.text()).not.toContain("Charaktere");
  });

  it("zeigt den Scan-Fortschritt und blendet ihn danach aus", async () => {
    let emit: ((event: { payload: unknown }) => void) | undefined;
    listen.mockImplementation((_name: string, handler: (e: { payload: unknown }) => void) => {
      emit = handler;
      return Promise.resolve(unlisten);
    });
    let resolveScan: ((value: unknown) => void) | undefined;
    invoke.mockImplementation((cmd: string) => {
      if (cmd === "detect_command") return Promise.resolve({ managed: ROOT, suggestions: [] });
      if (cmd === "inspect_wow_exe_command") return Promise.resolve(EXE);
      if (cmd === "client_state_command") return Promise.resolve("not-running");
      if (cmd === "wow_settings_command") return Promise.resolve({ loads_outdated_addons: true });
      if (cmd === "list_characters_command") return Promise.resolve([]);
      if (cmd === "scan_addons_command") return new Promise((r) => (resolveScan = r));
      return Promise.resolve(null);
    });

    const wrapper = mount(App);
    await flushPromises();
    await goTo(wrapper, "Addons");

    emit!({ payload: { done: 42, total: 242, current: "pfQuest" } });
    await flushPromises();
    expect(wrapper.text()).toContain("42 von 242");
    expect(wrapper.find("[role=progressbar]").attributes("aria-valuenow")).toBe("42");

    resolveScan!(SCAN);
    await flushPromises();
    expect(wrapper.find("[role=progressbar]").exists()).toBe(false);
  });

  it("scannt auch dann, wenn der Fortschritts-Listener scheitert", async () => {
    listen.mockRejectedValue(new Error("kein Event-System"));
    mockInvoke({ managed: ROOT, suggestions: [] }, { scan: SCAN });
    const wrapper = mount(App);
    await flushPromises();
    await goTo(wrapper, "Addons");
    expect(wrapper.text()).toContain("pfQuest");
  });

  it("meldet den Listener beim Zerstören wieder ab", async () => {
    mockInvoke({ managed: ROOT, suggestions: [] }, { scan: SCAN });
    const wrapper = mount(App);
    await flushPromises();
    wrapper.unmount();
    expect(unlisten).toHaveBeenCalled();
  });

  it("prüft den laufenden Client periodisch nach", async () => {
    // Der Client kann jederzeit gestartet werden, während die App offen ist.
    vi.useFakeTimers();
    mockInvoke({ managed: ROOT, suggestions: [] }, { scan: SCAN });
    const wrapper = mount(App);
    await flushPromises();
    const before = invoke.mock.calls.filter((c) => c[0] === "client_state_command").length;

    await vi.advanceTimersByTimeAsync(5000);
    const after = invoke.mock.calls.filter((c) => c[0] === "client_state_command").length;
    expect(after).toBeGreaterThan(before);

    wrapper.unmount();
    vi.useRealTimers();
  });

  it("warnt, wenn der Client aus dieser Installation läuft", async () => {
    invoke.mockImplementation((cmd: string) => {
      if (cmd === "detect_command") return Promise.resolve({ managed: ROOT, suggestions: [] });
      if (cmd === "inspect_wow_exe_command") return Promise.resolve(EXE);
      if (cmd === "client_state_command") return Promise.resolve("running-here");
      if (cmd === "wow_settings_command") return Promise.resolve({ loads_outdated_addons: true });
      if (cmd === "list_characters_command") return Promise.resolve([]);
      if (cmd === "scan_addons_command") return Promise.resolve(SCAN);
      return Promise.resolve(null);
    });
    const wrapper = mount(App);
    await flushPromises();
    expect(wrapper.text()).toContain("WoW läuft gerade");
    expect(wrapper.text()).toContain("Mit Einschränkungen");
  });

  it("fällt bei unlesbaren Client-Einstellungen auf den Vorgabewert zurück", async () => {
    // Vorgabewert heißt „lädt keine veralteten" — also werden sie ausgeblendet.
    // Nichts versprechen, was der Client nicht hält.
    invoke.mockImplementation((cmd: string) => {
      if (cmd === "detect_command") return Promise.resolve({ managed: ROOT, suggestions: [] });
      if (cmd === "inspect_wow_exe_command") return Promise.resolve(EXE);
      if (cmd === "client_state_command") return Promise.resolve("not-running");
      if (cmd === "wow_settings_command") return Promise.reject(new Error("Config.wtf kaputt"));
      if (cmd === "list_characters_command") return Promise.resolve([]);
      if (cmd === "scan_addons_command")
        return Promise.resolve({
          ...SCAN,
          addons: [{ ...SCAN.addons[0], id: "Alt", title: "Alt", interface: "11000" }],
        });
      return Promise.resolve(null);
    });
    const wrapper = mount(App);
    await flushPromises();
    await goTo(wrapper, "Addons");
    expect(wrapper.text()).toContain("ist hier ausgeblendet");
  });

  it("nimmt einen Fehler beim Client-Check als „läuft nicht“", async () => {
    // Ein fehlgeschlagener Check darf keine Warnung erfinden.
    invoke.mockImplementation((cmd: string) => {
      if (cmd === "detect_command") return Promise.resolve({ managed: ROOT, suggestions: [] });
      if (cmd === "inspect_wow_exe_command") return Promise.resolve(EXE);
      if (cmd === "client_state_command") return Promise.reject(new Error("keine Prozessliste"));
      if (cmd === "wow_settings_command") return Promise.resolve({ loads_outdated_addons: true });
      if (cmd === "list_characters_command") return Promise.resolve([]);
      if (cmd === "scan_addons_command") return Promise.resolve(SCAN);
      return Promise.resolve(null);
    });
    const wrapper = mount(App);
    await flushPromises();
    expect(wrapper.text()).toContain("Alles in Ordnung");
  });

  it("zeigt einen Hinweis, solange die Installation gesucht wird", async () => {
    invoke.mockImplementation(() => new Promise(() => {}));
    const wrapper = mount(App);
    await flushPromises();
    expect(wrapper.text()).toContain("Suche WoW-Installation");
  });
});

describe("App – Addon-Scan", () => {
  it("scannt nach erfolgreicher Erkennung und zeigt die Liste", async () => {
    mockInvoke({ managed: ROOT, suggestions: [] }, { scan: SCAN });
    const wrapper = mount(App);
    await flushPromises();

    expect(invoke).toHaveBeenCalledWith("scan_addons_command", { root: "/games/WoW" });
    await goTo(wrapper, "Addons");
    expect(wrapper.text()).toContain("pfQuest");
    expect(wrapper.text()).toContain("7c1d90ffaa11");
  });

  it("scannt nicht, wenn keine Installation verwaltet wird", async () => {
    // Vorschläge sind Ziele zum Hinverschieben, keine verwalteten Bestände.
    mockInvoke({ managed: null, suggestions: [OTHER] });
    const wrapper = mount(App);
    await flushPromises();

    expect(invoke).not.toHaveBeenCalledWith("scan_addons_command", expect.anything());
    // Auf ein Element der Tabelle prüfen, nicht auf "Addons" — das steht auch
    // im Menü, im Untertitel und im AddOns-Marker der Installation.
    expect(wrapper.text()).not.toContain("Nur Git-Checkouts");
  });

  it("meldet einen fehlgeschlagenen Scan, ohne die WoW-Erkennung zu verwerfen", async () => {
    mockInvoke({ managed: ROOT, suggestions: [] }, { scanFails: true });
    const wrapper = mount(App);
    await flushPromises();

    // Die erkannte Installation bleibt sichtbar …
    expect(wrapper.text()).toContain("/games/WoW");
    // … und der Fehler steht in der Addon-Ansicht.
    await goTo(wrapper, "Addons");
    expect(wrapper.text()).toContain("Addon-Scan fehlgeschlagen");
  });
});
