<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { type WowRoot, type WowExeInfo } from "./wow";

const props = defineProps<{
  root: WowRoot;
  exe?: WowExeInfo | null;
}>();

const { t } = useI18n();

/** Lokalisiertes Verdikt zur Exe-Identität. */
const identityText = computed(() => {
  const id = props.exe?.identity;
  if (!id) return "";
  switch (id.status) {
    case "official":
      return t("exe.identity.official", { version: id.version, locale: id.locale });
    case "modified":
      return t("exe.identity.modified", { version: id.claims_version });
    case "unknown-build":
      return t("exe.identity.unknownBuild");
    case "unknown":
      return t("exe.identity.unknown");
  }
});

/** Farbe des Verdikts — grün nur bei einer unveränderten Original-Exe. */
const identityTone = computed(() => {
  switch (props.exe?.identity.status) {
    case "official":
      return "text-verdict-ok dark:text-verdict-ok-dark";
    case "unknown":
      return "text-verdict-bad dark:text-verdict-bad-dark";
    default:
      return "text-verdict-warn dark:text-verdict-warn-dark";
  }
});

const markers = computed(() => [
  { label: "WoW.exe", ok: props.root.has_exe },
  { label: "MPQ", ok: props.root.has_mpq },
  { label: "Interface", ok: props.root.has_interface },
  { label: "AddOns", ok: props.root.has_addons },
]);
</script>

<template>
  <div class="tome-inset mb-3 px-4 py-3">
    <code class="tome-data block break-all">{{ root.path }}</code>
    <span class="font-display mt-1 inline-block text-[0.65rem] tracking-[0.15em] uppercase opacity-60">
      {{ root.method }}
    </span>

    <div class="mt-2 flex flex-wrap gap-2">
      <span
        v-for="marker in markers"
        :key="marker.label"
        :class="[
          'tome-badge',
          marker.ok ? 'text-verdict-ok dark:text-verdict-ok-dark' : 'opacity-40',
        ]"
        >{{ marker.label }}</span
      >
    </div>

    <div v-if="exe" class="mt-3 border-t border-gold-700/30 pt-2 text-sm">
      <p :class="['mb-2 font-semibold', identityTone]">{{ identityText }}</p>
      <dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5">
        <dt class="opacity-60">{{ t("exe.build") }}</dt>
        <dd class="tome-data">
          {{ exe.build ?? "—" }}
          <span v-if="exe.build_date" class="opacity-60">({{ exe.build_date }})</span>
        </dd>
        <dt class="opacity-60">{{ t("exe.size") }}</dt>
        <dd class="tome-data">{{ exe.size_bytes.toLocaleString() }} B</dd>
        <dt class="opacity-60">SHA-1</dt>
        <dd class="tome-data break-all">{{ exe.sha1 }}</dd>
        <dt class="opacity-60">MD5</dt>
        <dd class="tome-data break-all">{{ exe.md5 }}</dd>
      </dl>
    </div>
  </div>
</template>
