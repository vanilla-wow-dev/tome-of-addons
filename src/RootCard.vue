<script setup lang="ts">
import { type WowRoot, type WowExeInfo, identityLabel } from "./wow";

defineProps<{
  root: WowRoot;
  exe?: WowExeInfo | null;
}>();
</script>

<template>
  <div class="root">
    <code class="path">{{ root.path }}</code>
    <span class="method">{{ root.method }}</span>
    <span class="markers">
      <span :class="{ ok: root.has_exe }">WoW.exe</span>
      <span :class="{ ok: root.has_mpq }">MPQ</span>
      <span :class="{ ok: root.has_interface }">Interface</span>
      <span :class="{ ok: root.has_addons }">AddOns</span>
    </span>

    <div v-if="exe" class="exe">
      <p :class="['identity', exe.identity.status]">{{ identityLabel(exe.identity) }}</p>
      <dl>
        <dt>Build</dt>
        <dd>
          {{ exe.build ?? "—" }}
          <span v-if="exe.build_date" class="dim">({{ exe.build_date }})</span>
        </dd>
        <dt>Größe</dt>
        <dd>{{ exe.size_bytes.toLocaleString() }} B</dd>
        <dt>SHA-1</dt>
        <dd class="hash">{{ exe.sha1 }}</dd>
        <dt>MD5</dt>
        <dd class="hash">{{ exe.md5 }}</dd>
      </dl>
    </div>
  </div>
</template>

<style scoped>
.root {
  padding: 0.75em 1em;
  border: 1px solid #d0d0d0;
  border-radius: 8px;
  margin-bottom: 0.5em;
}

.path {
  display: block;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.85em;
  word-break: break-all;
}

.method {
  display: inline-block;
  margin-top: 0.4em;
  font-size: 0.75em;
  color: #888;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.markers {
  display: flex;
  gap: 0.5em;
  margin-top: 0.5em;
  flex-wrap: wrap;
}

.markers span {
  font-size: 0.75em;
  padding: 0.15em 0.5em;
  border-radius: 4px;
  background: rgba(0, 0, 0, 0.06);
  color: #999;
}

.markers span.ok {
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
</style>
