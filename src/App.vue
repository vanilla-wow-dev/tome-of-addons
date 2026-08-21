<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, shallowRef } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getVersion } from "@tauri-apps/api/app";
import { check, Update } from "@tauri-apps/plugin-updater";
import { relaunch, exit } from "@tauri-apps/plugin-process";
import { type WowRoot, type WowExeInfo, type Detection, fmtBytes } from "./wow";
import {
  installationHealth,
  type AddonScan,
  type Character,
  type ClientState,
  type ScanProgress,
  type WowSettings,
} from "./addons";
import { SUPPORTED_LOCALES, setLocale, type Locale } from "./i18n";
import SideBar from "./SideBar.vue";
import WowView from "./WowView.vue";
import AddonTable from "./AddonTable.vue";
import CharacterView from "./CharacterView.vue";

const { t, locale } = useI18n();

function onLanguageChange(event: Event) {
  setLocale((event.target as HTMLSelectElement).value as Locale);
}

/** Aktive Ansicht: "wow", "addons" oder der Pfad einer AddOns.txt. */
const view = ref("wow");

const detection = ref<Detection | null>(null);
const wowError = ref("");
const exeInfo = ref<Record<string, WowExeInfo>>({});
const relocateMsg = ref("");
const clientState = ref<ClientState>("not-running");

const addonScan = ref<AddonScan | null>(null);
const addonError = ref("");
const addonBusy = ref(false);
const scanProgress = ref<ScanProgress | null>(null);
const characters = ref<Character[]>([]);
const settings = ref<WowSettings>({ loads_outdated_addons: false });
let unlistenProgress: UnlistenFn | undefined;
let clientTimer: ReturnType<typeof setInterval> | undefined;

const managed = computed(() => detection.value?.managed ?? null);

/** Interface-Version des Clients — Bezugswert für die Veraltet-Erkennung. */
const clientInterface = computed(
  () => (managed.value && exeInfo.value[managed.value.path]?.interface_version) || null,
);

const health = computed(() =>
  installationHealth(
    managed.value,
    managed.value ? exeInfo.value[managed.value.path] : null,
    clientState.value,
  ),
);

const activeCharacter = computed(
  () => characters.value.find((character) => character.path === view.value) ?? null,
);

async function detectWow() {
  wowError.value = "";
  detection.value = null;
  relocateMsg.value = "";
  exeInfo.value = {};
  try {
    const result = await invoke<Detection>("detect_command");
    const roots = [result.managed, ...result.suggestions].filter((r): r is WowRoot => r !== null);
    await Promise.all(roots.map((r) => inspectExe(r.path)));
    detection.value = result;
    if (result.managed) {
      await refreshClientState(result.managed.path);
      await loadSettings(result.managed.path);
      await loadCharacters(result.managed.path);
      await scanAddons(result.managed.path);
    }
  } catch (err) {
    wowError.value = t("wow.searchFailed", { err: String(err) });
  }
}

async function inspectExe(root: string) {
  try {
    exeInfo.value[root] = await invoke<WowExeInfo>("inspect_wow_exe_command", { root });
  } catch {
    // WoW.exe nicht lesbar — Detailbereich bleibt einfach leer.
  }
}

/**
 * Der Client kann jederzeit gestartet oder beendet werden, während die App
 * offen ist — deshalb periodisch statt einmalig.
 */
async function refreshClientState(root: string) {
  try {
    clientState.value = await invoke<ClientState>("client_state_command", { root });
  } catch {
    clientState.value = "not-running";
  }
}

/**
 * Client-Einstellungen. Bei einem Fehler bleibt es beim Client-Vorgabewert
 * „veraltete werden nicht geladen" — nichts versprechen, was nicht gilt.
 */
async function loadSettings(root: string) {
  try {
    settings.value = await invoke<WowSettings>("wow_settings_command", { root });
  } catch {
    settings.value = { loads_outdated_addons: false };
  }
}

async function loadCharacters(root: string) {
  try {
    characters.value = await invoke<Character[]>("list_characters_command", { root });
  } catch {
    characters.value = [];
  }
}

async function scanAddons(root: string) {
  addonBusy.value = true;
  addonError.value = "";
  scanProgress.value = null;
  try {
    addonScan.value = await invoke<AddonScan>("scan_addons_command", { root });
  } catch (err) {
    addonScan.value = null;
    addonError.value = t("addons.scanFailed", { err: String(err) });
  } finally {
    addonBusy.value = false;
    scanProgress.value = null;
  }
}

async function relocateInto(targetRoot: string) {
  relocateMsg.value = t("relocate.inProgress");
  try {
    const dest = await invoke<string>("relocate_into_command", { targetRoot });
    relocateMsg.value = t("relocate.done", { dest });
    await exit(0);
  } catch (err) {
    relocateMsg.value = t("relocate.failed", { err: String(err) });
  }
}

type Status = "idle" | "available" | "downloading" | "ready" | "error";

// Intervall des automatischen Hintergrund-Checks (Start + alle 24 h).
const UPDATE_CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000;
// Der laufende Client wechselt oft und wird deshalb deutlich häufiger geprüft.
const CLIENT_CHECK_INTERVAL_MS = 5000;

const version = ref("");
const status = ref<Status>("idle");
const message = ref("");
// shallowRef statt ref: der Update-Wert ist eine Tauri-Klasseninstanz mit
// privaten Feldern (#rid). Ein reaktiver Proxy (ref) bricht deren Methoden
// ("Cannot read private member …"); shallowRef hält die rohe Instanz.
const pendingUpdate = shallowRef<Update | null>(null);
const progress = ref<{ downloaded: number; total: number | null }>({ downloaded: 0, total: null });
let updateTimer: ReturnType<typeof setInterval> | undefined;

onMounted(async () => {
  // Bewusst nicht nacheinander: die Version steht sofort, die Erkennung läuft
  // parallel, und der Update-Check darf die Anzeige nie aufhalten.
  getVersion().then((v) => (version.value = v));
  // Vor der Erkennung registrieren, damit keine frühen Fortschritts-Events
  // verlorengehen — aber abgesichert: ein fehlschlagender Listener darf den
  // Scan nicht verhindern, dann bleibt der Balken eben unbestimmt.
  try {
    unlistenProgress = await listen<ScanProgress>("addon-scan-progress", (event) => {
      scanProgress.value = event.payload;
    });
  } catch {
    unlistenProgress = undefined;
  }
  void checkForUpdate();
  updateTimer = setInterval(checkForUpdate, UPDATE_CHECK_INTERVAL_MS);
  clientTimer = setInterval(() => {
    if (managed.value) void refreshClientState(managed.value.path);
  }, CLIENT_CHECK_INTERVAL_MS);
  await detectWow();
});

onUnmounted(() => {
  if (updateTimer) clearInterval(updateTimer);
  if (clientTimer) clearInterval(clientTimer);
  unlistenProgress?.();
});

// Automatischer Hintergrund-Check: meldet NUR, wenn ein Update verfügbar ist.
// „Kein Update" und Fehler bleiben still (kein Nerven) — Fehler nur ins Log.
async function checkForUpdate() {
  try {
    const update = await check();
    if (update) {
      pendingUpdate.value = update;
      status.value = "available";
      message.value = t("update.available", { version: update.version, current: version.value });
    }
  } catch (err) {
    console.error("Update-Check fehlgeschlagen:", err);
  }
}

async function installUpdate() {
  if (!pendingUpdate.value) return;
  status.value = "downloading";
  // Ohne diese Zeile stünde im Banner weiter „Version X verfügbar", während
  // schon geladen wird — der frühere separate Block hatte den Text selbst.
  message.value = t("update.downloading");
  progress.value = { downloaded: 0, total: null };
  try {
    await pendingUpdate.value.downloadAndInstall((event) => {
      if (event.event === "Started") {
        progress.value.total = event.data.contentLength ?? null;
      } else if (event.event === "Progress") {
        progress.value.downloaded += event.data.chunkLength;
      } else if (event.event === "Finished") {
        status.value = "ready";
        message.value = t("update.ready");
      }
    });
  } catch (err) {
    status.value = "error";
    message.value = t("update.downloadFailed", { err: String(err) });
  }
}

async function restartNow() {
  await relaunch();
}
</script>

<template>
  <!-- Volle Fensterfläche, Scrollen ausschließlich innen: die Kopfzeile und
       die Seitenleiste sollen beim Blättern stehen bleiben. `min-w` verhindert,
       dass die Tabelle unter einer bestimmten Breite unlesbar zusammenfällt —
       schmaler wird das Fenster ohnehin nicht (minWidth in tauri.conf.json). -->
  <main class="relative z-[1] flex h-full min-w-[860px] flex-col gap-4 px-6 py-4">
    <header class="flex shrink-0 items-end gap-4">
      <div>
        <h1 class="text-3xl text-gold-700 dark:text-gold-300">Tome of Addons</h1>
        <p class="text-sm italic opacity-70">{{ t("tagline") }}</p>
      </div>
      <p data-testid="version" class="tome-data mb-1 opacity-50">v{{ version || "…" }}</p>
      <label class="ml-auto text-sm">
        <span class="sr-only">{{ t("language") }}</span>
        <select
          :value="locale"
          aria-label="language"
          class="tome-input text-xs"
          @change="onLanguageChange"
        >
          <option v-for="l in SUPPORTED_LOCALES" :key="l" :value="l">{{ l.toUpperCase() }}</option>
        </select>
      </label>
    </header>

    <!-- Der Update-Banner steht über allem, damit er in jeder Ansicht sichtbar
         bleibt statt an eine davon gebunden zu sein. -->
    <div v-if="message" class="tome-panel flex shrink-0 flex-wrap items-center gap-4 p-4">
      <p>{{ message }}</p>
      <p v-if="status === 'downloading'" class="tome-data opacity-70">
        {{ fmtBytes(progress.downloaded)
        }}<span v-if="progress.total"> / {{ fmtBytes(progress.total) }}</span>
      </p>
      <button
        v-if="status === 'available'"
        type="button"
        class="tome-button ml-auto"
        @click="installUpdate"
      >
        {{ t("update.download") }}
      </button>
      <button v-if="status === 'ready'" type="button" class="tome-button ml-auto" @click="restartNow">
        {{ t("update.restart") }}
      </button>
    </div>

    <div class="flex min-h-0 flex-1 gap-6 pb-2">
      <SideBar
        :health="health"
        :addons="addonScan?.addons ?? []"
        :characters="characters"
        :view="view"
        @select="view = $event"
      />

      <div class="tome-panel flex min-h-0 min-w-0 flex-1 flex-col p-5">
        <WowView
          v-if="view === 'wow'"
          class="tome-scroll min-h-0 flex-1"
          :detection="detection"
          :exe-info="exeInfo"
          :health="health"
          :error="wowError"
          :relocate-message="relocateMsg"
          @relocate="relocateInto"
        />

        <template v-else>
          <div v-if="addonBusy" class="tome-scroll min-h-0 flex-1">
            <p class="tome-heading mb-3">{{ t("addons.title") }}</p>
            <p class="mb-2 text-sm opacity-70">
              {{
                scanProgress
                  ? t("addons.scanProgress", {
                      done: scanProgress.done,
                      total: scanProgress.total,
                    })
                  : t("addons.scanning")
              }}
              <span v-if="scanProgress" class="tome-data ml-2 opacity-60">{{
                scanProgress.current
              }}</span>
            </p>
            <div
              class="h-2 w-full overflow-hidden rounded-sm border border-gold-700/40 bg-gold-500/10"
              role="progressbar"
              :aria-valuenow="scanProgress?.done ?? 0"
              :aria-valuemin="0"
              :aria-valuemax="scanProgress?.total ?? 0"
            >
              <div
                class="h-full bg-gold-500/70 transition-[width] duration-150"
                :style="{
                  width: scanProgress ? `${(scanProgress.done / scanProgress.total) * 100}%` : '0%',
                }"
              />
            </div>
          </div>

          <p v-else-if="addonError" class="text-verdict-bad dark:text-verdict-bad-dark">
            {{ addonError }}
          </p>

          <CharacterView
            v-else-if="addonScan && activeCharacter"
            :key="activeCharacter.path"
            class="flex min-h-0 flex-1 flex-col"
            :character="activeCharacter"
            :scan="addonScan"
            :client-interface="clientInterface"
            :loads-outdated="settings.loads_outdated_addons"
          />

          <section v-else-if="addonScan" class="flex min-h-0 flex-1 flex-col">
            <h2 class="tome-heading mb-4 shrink-0">{{ t("addons.title") }}</h2>
            <AddonTable
              key="addons"
              class="min-h-0 flex-1"
              :scan="addonScan"
              :client-interface="clientInterface"
              :loads-outdated="settings.loads_outdated_addons"
            />
          </section>
        </template>
      </div>
    </div>
  </main>
</template>
