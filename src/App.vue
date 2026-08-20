<script setup lang="ts">
import { onMounted, onUnmounted, ref, shallowRef } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { check, Update } from "@tauri-apps/plugin-updater";
import { relaunch, exit } from "@tauri-apps/plugin-process";
import { type WowRoot, type WowExeInfo, type Detection, fmtBytes } from "./wow";
import { type AddonScan } from "./addons";
import { SUPPORTED_LOCALES, setLocale, type Locale } from "./i18n";
import RootCard from "./RootCard.vue";
import AddonTable from "./AddonTable.vue";

const { t, locale } = useI18n();

function onLanguageChange(event: Event) {
  setLocale((event.target as HTMLSelectElement).value as Locale);
}

const managed = ref<WowRoot | null>(null);
const suggestions = ref<WowRoot[]>([]);
const wowScanned = ref(false);
const wowError = ref("");
const exeInfo = ref<Record<string, WowExeInfo>>({});
const relocateMsg = ref("");

const addonScan = ref<AddonScan | null>(null);
const addonError = ref("");
const addonBusy = ref(false);

async function detectWow() {
  wowError.value = "";
  wowScanned.value = false;
  relocateMsg.value = "";
  exeInfo.value = {};
  try {
    const detection = await invoke<Detection>("detect_command");
    managed.value = detection.managed;
    suggestions.value = detection.suggestions;
    wowScanned.value = true;
    const roots = [detection.managed, ...detection.suggestions].filter(
      (r): r is WowRoot => r !== null,
    );
    await Promise.all(roots.map((r) => inspectExe(r.path)));
    // Gescannt wird nur die verwaltete Installation — Vorschläge sind Ziele
    // zum Hinverschieben, nicht verwaltete Bestände.
    if (detection.managed) await scanAddons(detection.managed.path);
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

async function scanAddons(root: string) {
  addonBusy.value = true;
  addonError.value = "";
  try {
    addonScan.value = await invoke<AddonScan>("scan_addons_command", { root });
  } catch (err) {
    addonScan.value = null;
    addonError.value = t("addons.scanFailed", { err: String(err) });
  } finally {
    addonBusy.value = false;
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
  version.value = await getVersion();
  await detectWow();
  await checkForUpdate();
  updateTimer = setInterval(checkForUpdate, UPDATE_CHECK_INTERVAL_MS);
});

onUnmounted(() => {
  if (updateTimer) clearInterval(updateTimer);
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
  <main class="relative z-[1] mx-auto max-w-5xl px-6 py-[5vh]">
    <label class="absolute top-[5vh] right-6 text-sm">
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

    <header class="text-center">
      <h1 class="text-4xl text-gold-700 dark:text-gold-300">Tome of Addons</h1>
      <p class="mt-1 italic opacity-70">{{ t("tagline") }}</p>
      <p data-testid="version" class="tome-data mt-1 opacity-50">v{{ version || "…" }}</p>
    </header>

    <section class="mx-auto mt-8 max-w-2xl text-left">
      <p v-if="wowError" class="text-verdict-bad dark:text-verdict-bad-dark my-4">{{ wowError }}</p>

      <template v-else-if="wowScanned">
        <!-- Zustand 1: verankert — diese Installation wird verwaltet. -->
        <div v-if="managed">
          <p class="tome-heading mb-2">{{ t("wow.managed") }}</p>
          <RootCard :root="managed" :exe="exeInfo[managed.path]" />

          <div v-if="suggestions.length" class="mt-6 border-t border-dashed border-gold-700/40 pt-4">
            <p class="tome-heading mb-2">{{ t("wow.others") }}</p>
            <p class="mb-3 text-sm opacity-60">{{ t("wow.moveHint") }}</p>
            <div v-for="s in suggestions" :key="s.path" class="mb-4">
              <RootCard :root="s" :exe="exeInfo[s.path]" />
              <button type="button" class="tome-button" @click="relocateInto(s.path)">
                {{ t("wow.moveHere") }}
              </button>
            </div>
          </div>
        </div>

        <!-- Zustand 2: nicht verankert, aber Installationen erkannt. -->
        <div v-else-if="suggestions.length">
          <p class="text-verdict-warn dark:text-verdict-warn-dark my-4">{{ t("wow.unanchored") }}</p>
          <div v-for="s in suggestions" :key="s.path" class="mb-4">
            <RootCard :root="s" :exe="exeInfo[s.path]" />
            <button type="button" class="tome-button" @click="relocateInto(s.path)">
              {{ t("wow.moveHere") }}
            </button>
          </div>
        </div>

        <!-- Zustand 3: nichts gefunden. -->
        <p v-else class="my-4">{{ t("wow.none") }}</p>

        <p v-if="relocateMsg" class="my-4">{{ relocateMsg }}</p>
      </template>
    </section>

    <p v-if="addonBusy" class="mt-6 text-center opacity-70">{{ t("addons.scanning") }}</p>
    <p v-else-if="addonError" class="text-verdict-bad dark:text-verdict-bad-dark mt-6 text-center">
      {{ addonError }}
    </p>
    <AddonTable v-else-if="addonScan" :scan="addonScan" />

    <p v-if="message" class="my-4 text-center">{{ message }}</p>

    <div v-if="status === 'available'" class="tome-panel mx-auto mt-6 max-w-md p-4 text-center">
      <button type="button" class="tome-button" @click="installUpdate">
        {{ t("update.download") }}
      </button>
    </div>

    <div v-if="status === 'downloading'" class="tome-panel mx-auto mt-6 max-w-md p-4 text-center">
      <p>
        {{ t("update.downloading") }}
        {{ fmtBytes(progress.downloaded) }}
        <span v-if="progress.total"> / {{ fmtBytes(progress.total) }}</span>
      </p>
    </div>

    <div v-if="status === 'ready'" class="tome-panel mx-auto mt-6 max-w-md p-4 text-center">
      <button type="button" class="tome-button" @click="restartNow">
        {{ t("update.restart") }}
      </button>
    </div>
  </main>
</template>
