<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { check, Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

interface WowRoot {
  path: string;
  has_exe: boolean;
  has_mpq: boolean;
  has_interface: boolean;
  has_addons: boolean;
  method: string;
}

type ExeIdentity =
  | { status: "official"; version: string; locale: string }
  | { status: "modified"; claims_version: string }
  | { status: "unknown-build" }
  | { status: "unknown" };

interface WowExeInfo {
  path: string;
  size_bytes: number;
  build: number | null;
  build_date: string | null;
  sha1: string;
  md5: string;
  identity: ExeIdentity;
}

const wowRoots = ref<WowRoot[]>([]);
const wowScanned = ref(false);
const wowError = ref("");
const exeInfo = ref<Record<string, WowExeInfo>>({});

async function detectWow() {
  wowError.value = "";
  wowScanned.value = false;
  exeInfo.value = {};
  try {
    wowRoots.value = await invoke<WowRoot[]>("detect_wow_roots_command");
    wowScanned.value = true;
    await Promise.all(wowRoots.value.map((r) => inspectExe(r.path)));
  } catch (err) {
    wowError.value = `WoW-Suche fehlgeschlagen: ${err}`;
  }
}

async function inspectExe(root: string) {
  try {
    exeInfo.value[root] = await invoke<WowExeInfo>("inspect_wow_exe_command", { root });
  } catch {
    // WoW.exe nicht lesbar — Detailbereich bleibt einfach leer.
  }
}

function identityLabel(id: ExeIdentity): string {
  switch (id.status) {
    case "official":
      return `✓ Offiziell ${id.version} (${id.locale})`;
    case "modified":
      return `⚠ Modifiziert (gibt sich als ${id.claims_version} aus)`;
    case "unknown-build":
      return "⚠ Unbekannter Build (kein offizieller Referenz-Hash)";
    case "unknown":
      return "✗ Kein erkennbarer WoW-Client";
  }
}

type Status = "idle" | "checking" | "uptodate" | "available" | "downloading" | "ready" | "error";

const version = ref("");
const status = ref<Status>("idle");
const message = ref("");
const pendingUpdate = ref<Update | null>(null);
const progress = ref<{ downloaded: number; total: number | null }>({ downloaded: 0, total: null });

onMounted(async () => {
  version.value = await getVersion();
  await detectWow();
});

async function checkForUpdate() {
  status.value = "checking";
  message.value = "";
  try {
    const update = await check();
    if (update) {
      pendingUpdate.value = update;
      status.value = "available";
      message.value = `Version ${update.version} verfügbar (aktuell ${version.value}).`;
    } else {
      status.value = "uptodate";
      message.value = "Du bist auf dem neuesten Stand.";
    }
  } catch (err) {
    status.value = "error";
    message.value = `Update-Check fehlgeschlagen: ${err}`;
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
        message.value = "Update installiert. Neustart erforderlich.";
      }
    });
  } catch (err) {
    status.value = "error";
    message.value = `Download fehlgeschlagen: ${err}`;
  }
}

async function restartNow() {
  await relaunch();
}

function fmtBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}
</script>

<template>
  <main class="container">
    <h1>Tome of Addons</h1>
    <p class="tagline">The curated tome of WoW 1.12.1 addons.</p>
    <p class="version">v{{ version || "…" }}</p>

    <div class="actions">
      <button
        type="button"
        :disabled="status === 'checking' || status === 'downloading'"
        @click="checkForUpdate"
      >
        {{ status === "checking" ? "Prüfe…" : "Auf Updates prüfen" }}
      </button>
      <button type="button" @click="detectWow">WoW-Verzeichnis suchen</button>
    </div>

    <section class="wow">
      <p v-if="wowError" class="message error">{{ wowError }}</p>
      <template v-else-if="wowScanned">
        <p v-if="wowRoots.length === 0" class="message">
          Keine WoW-1.12.1-Installation gefunden.
        </p>
        <ul v-else class="roots">
          <li v-for="root in wowRoots" :key="root.path" class="root">
            <code class="path">{{ root.path }}</code>
            <span class="method">{{ root.method }}</span>
            <span class="markers">
              <span :class="{ ok: root.has_exe }">WoW.exe</span>
              <span :class="{ ok: root.has_mpq }">MPQ</span>
              <span :class="{ ok: root.has_interface }">Interface</span>
              <span :class="{ ok: root.has_addons }">AddOns</span>
            </span>

            <div v-if="exeInfo[root.path]" class="exe">
              <p :class="['identity', exeInfo[root.path].identity.status]">
                {{ identityLabel(exeInfo[root.path].identity) }}
              </p>
              <dl>
                <dt>Build</dt>
                <dd>{{ exeInfo[root.path].build ?? "—" }}
                  <span v-if="exeInfo[root.path].build_date" class="dim">
                    ({{ exeInfo[root.path].build_date }})</span>
                </dd>
                <dt>Größe</dt>
                <dd>{{ exeInfo[root.path].size_bytes.toLocaleString() }} B</dd>
                <dt>SHA-1</dt>
                <dd class="hash">{{ exeInfo[root.path].sha1 }}</dd>
                <dt>MD5</dt>
                <dd class="hash">{{ exeInfo[root.path].md5 }}</dd>
              </dl>
            </div>
          </li>
        </ul>
      </template>
    </section>

    <p v-if="message" :class="['message', status]">{{ message }}</p>

    <div v-if="status === 'available'" class="banner">
      <button type="button" @click="installUpdate">Update herunterladen &amp; installieren</button>
    </div>

    <div v-if="status === 'downloading'" class="banner">
      <p>
        Lade…
        {{ fmtBytes(progress.downloaded) }}
        <span v-if="progress.total"> / {{ fmtBytes(progress.total) }}</span>
      </p>
    </div>

    <div v-if="status === 'ready'" class="banner ready">
      <button type="button" @click="restartNow">Jetzt neustarten</button>
    </div>
  </main>
</template>

<style scoped>
.container {
  max-width: 640px;
  margin: 0 auto;
  padding: 6vh 2em;
  text-align: center;
}

.tagline {
  color: #888;
  margin-top: -0.5em;
  font-style: italic;
}

.version {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  color: #888;
}

.actions {
  margin: 2em 0 1em;
  display: flex;
  gap: 0.75em;
  justify-content: center;
}

.wow {
  margin: 1.5em auto;
  max-width: 560px;
  text-align: left;
}

.roots {
  list-style: none;
  padding: 0;
}

.root {
  padding: 0.75em 1em;
  border: 1px solid #d0d0d0;
  border-radius: 8px;
  margin-bottom: 0.75em;
}

.root .path {
  display: block;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.85em;
  word-break: break-all;
}

.root .method {
  display: inline-block;
  margin-top: 0.4em;
  font-size: 0.75em;
  color: #888;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.root .markers {
  display: flex;
  gap: 0.5em;
  margin-top: 0.5em;
  flex-wrap: wrap;
}

.root .markers span {
  font-size: 0.75em;
  padding: 0.15em 0.5em;
  border-radius: 4px;
  background: rgba(0, 0, 0, 0.06);
  color: #999;
}

.root .markers span.ok {
  background: rgba(39, 174, 96, 0.15);
  color: #1e8449;
}

.exe {
  margin-top: 0.75em;
  padding-top: 0.6em;
  border-top: 1px solid #e0e0e0;
  font-size: 0.8em;
}

.exe .identity {
  font-weight: 600;
  margin: 0 0 0.5em;
}

.exe .identity.official {
  color: #1e8449;
}

.exe .identity.modified,
.exe .identity.unknown-build {
  color: #b9770e;
}

.exe .identity.unknown {
  color: #c0392b;
}

.exe dl {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 0.15em 0.75em;
  margin: 0;
}

.exe dt {
  color: #888;
}

.exe dd {
  margin: 0;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  word-break: break-all;
}

.exe dd.hash {
  font-size: 0.92em;
}

.exe .dim {
  color: #888;
  font-family: inherit;
}

.message {
  margin: 1em 0;
}

.message.error {
  color: #c0392b;
}

.message.uptodate {
  color: #27ae60;
}

.banner {
  margin: 1.5em auto;
  padding: 1em;
  border: 1px solid #d0d0d0;
  border-radius: 8px;
  max-width: 420px;
}

.banner.ready {
  border-color: #27ae60;
  background: rgba(39, 174, 96, 0.1);
}
</style>

<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 1.5;
  color: #1a1a1a;
  background-color: #f6f6f6;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f0f0f0;
    background-color: #1f1f1f;
  }
  .banner {
    border-color: #444;
  }
}

button {
  border-radius: 8px;
  border: 1px solid #c0c0c0;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  background-color: #ffffff;
  color: #1a1a1a;
  cursor: pointer;
  transition: border-color 0.2s, background-color 0.2s;
}

button:hover:not(:disabled) {
  border-color: #396cd8;
}

button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

@media (prefers-color-scheme: dark) {
  button {
    background-color: #2a2a2a;
    color: #f0f0f0;
    border-color: #444;
  }
}
</style>
