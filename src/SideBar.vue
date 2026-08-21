<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { activeCount, type Addon, type Character, type HealthVerdict } from "./addons";

defineProps<{
  health: HealthVerdict;
  addons: Addon[];
  characters: Character[];
  /** Aktive Ansicht: "wow", "addons" oder der Pfad einer AddOns.txt. */
  view: string;
}>();
const emit = defineEmits<{ select: [view: string] }>();
const { t } = useI18n();

/**
 * Nur die Installation trägt eine Ampel. Für Addons und Charaktere steht die
 * Zahl daneben — sie sagt mehr als ein Punkt und erzeugt kein Dauer-Gelb.
 */
const TONE: Record<string, string> = {
  ok: "text-verdict-ok dark:text-verdict-ok-dark",
  warn: "text-verdict-warn dark:text-verdict-warn-dark",
  error: "text-verdict-bad dark:text-verdict-bad-dark",
};
</script>

<template>
  <nav class="flex w-60 shrink-0 flex-col gap-1" :aria-label="t('nav.label')">
    <button
      type="button"
      :class="[
        'flex cursor-pointer items-center gap-2 rounded-sm px-3 py-2 text-left text-sm',
        view === 'wow' ? 'bg-gold-500/15 font-semibold' : 'hover:bg-gold-500/8',
      ]"
      :aria-current="view === 'wow' ? 'page' : undefined"
      @click="emit('select', 'wow')"
    >
      <span :class="TONE[health.level]" aria-hidden="true">●</span>
      <span>{{ t("nav.wow") }}</span>
      <span class="sr-only">{{ t(`nav.health.${health.level}`) }}</span>
    </button>

    <button
      type="button"
      :class="[
        'flex cursor-pointer items-center gap-2 rounded-sm px-3 py-2 text-left text-sm',
        view === 'addons' ? 'bg-gold-500/15 font-semibold' : 'hover:bg-gold-500/8',
      ]"
      :aria-current="view === 'addons' ? 'page' : undefined"
      @click="emit('select', 'addons')"
    >
      <span class="opacity-30" aria-hidden="true">●</span>
      <span>{{ t("nav.addons") }}</span>
      <span class="tome-data ml-auto opacity-60">{{ addons.length }}</span>
    </button>

    <!-- Die festen Einträge bleiben stehen, nur die Charakterliste blättert —
         bei sieben Accounts wird sie sonst länger als das Fenster. -->
    <template v-if="characters.length">
      <p class="tome-heading mt-5 mb-1 shrink-0 px-1">{{ t("nav.characters") }}</p>
      <div class="tome-scroll min-h-0 flex-1">
      <button
        v-for="character in characters"
        :key="character.path"
        type="button"
        :class="[
          'flex w-full cursor-pointer items-center gap-2 rounded-sm px-3 py-1.5 text-left text-sm',
          view === character.path ? 'bg-gold-500/15 font-semibold' : 'hover:bg-gold-500/8',
        ]"
        :aria-current="view === character.path ? 'page' : undefined"
        :title="character.label"
        @click="emit('select', character.path)"
      >
        <span class="truncate">{{ character.name }}</span>
        <span class="tome-data ml-auto opacity-60">{{ activeCount(character, addons) }}</span>
      </button>
      </div>
    </template>
  </nav>
</template>
