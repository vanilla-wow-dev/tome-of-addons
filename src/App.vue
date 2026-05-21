<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { check, Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

type Status = "idle" | "checking" | "uptodate" | "available" | "downloading" | "ready" | "error";

const version = ref("");
const status = ref<Status>("idle");
const message = ref("");
const pendingUpdate = ref<Update | null>(null);
const progress = ref<{ downloaded: number; total: number | null }>({ downloaded: 0, total: null });

onMounted(async () => {
  version.value = await getVersion();
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
    </div>

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
