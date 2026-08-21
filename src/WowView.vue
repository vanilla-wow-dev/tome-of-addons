<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { Detection, WowExeInfo } from "./wow";
import type { HealthVerdict } from "./addons";
import RootCard from "./RootCard.vue";

defineProps<{
  detection: Detection | null;
  exeInfo: Record<string, WowExeInfo>;
  health: HealthVerdict;
  error: string;
  relocateMessage: string;
}>();
const emit = defineEmits<{ relocate: [target: string] }>();
const { t } = useI18n();

const TONE: Record<string, string> = {
  ok: "text-verdict-ok dark:text-verdict-ok-dark",
  warn: "text-verdict-warn dark:text-verdict-warn-dark",
  error: "text-verdict-bad dark:text-verdict-bad-dark",
};
</script>

<template>
  <section>
    <h2 class="tome-heading mb-4">{{ t("nav.wow") }}</h2>

    <p v-if="error" class="text-verdict-bad dark:text-verdict-bad-dark my-4">{{ error }}</p>

    <!-- Ohne diese Zeile bliebe der Bereich während der Erkennung leer und die
         App wirkte eingefroren. -->
    <p v-else-if="!detection" class="my-4 animate-pulse opacity-60">{{ t("wow.searching") }}</p>

    <template v-else>
      <!-- Die Ampel wird ausgeschrieben, statt sie nur zu färben: Farbe allein
           ist für Farbfehlsichtige keine Information. -->
      <p :class="['mb-4 font-semibold', TONE[health.level]]">
        <span aria-hidden="true">●</span>
        {{ t(`nav.health.${health.level}`) }}
      </p>
      <ul v-if="health.reasons.length" class="mb-4 space-y-1 text-sm">
        <li v-for="reason in health.reasons" :key="reason" class="flex gap-2">
          <span class="text-verdict-warn dark:text-verdict-warn-dark" aria-hidden="true">⚠</span>
          <span>{{ t(reason) }}</span>
        </li>
      </ul>

      <div v-if="detection.managed">
        <p class="tome-heading mb-2">{{ t("wow.managed") }}</p>
        <RootCard :root="detection.managed" :exe="exeInfo[detection.managed.path]" />
      </div>

      <p v-else-if="!detection.suggestions.length" class="my-4">{{ t("wow.none") }}</p>
      <p v-else class="text-verdict-warn dark:text-verdict-warn-dark my-4">
        {{ t("wow.unanchored") }}
      </p>

      <div
        v-if="detection.suggestions.length"
        class="mt-6 border-t border-dashed border-gold-700/40 pt-4"
      >
        <p class="tome-heading mb-2">{{ t("wow.others") }}</p>
        <p class="mb-3 text-sm opacity-60">{{ t("wow.moveHint") }}</p>
        <div v-for="s in detection.suggestions" :key="s.path" class="mb-4">
          <RootCard :root="s" :exe="exeInfo[s.path]" />
          <button type="button" class="tome-button" @click="emit('relocate', s.path)">
            {{ t("wow.moveHere") }}
          </button>
        </div>
      </div>

      <p v-if="relocateMessage" class="my-4">{{ relocateMessage }}</p>
    </template>
  </section>
</template>
