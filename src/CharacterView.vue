<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { interfaceStatus, stateFor, type AddonScan, type Character } from "./addons";
import AddonTable from "./AddonTable.vue";

const props = defineProps<{
  character: Character;
  scan: AddonScan;
  clientInterface: string | null;
}>();
const { t } = useI18n();

const tally = computed(() => {
  let on = 0;
  let off = 0;
  let unseen = 0;
  for (const addon of props.scan.addons) {
    const state = stateFor(props.character, addon);
    if (state === true) on += 1;
    else if (state === false) off += 1;
    else unseen += 1;
  }
  return { on, off, unseen };
});

/**
 * Der eigentlich interessante Fall: aktiv, aber veraltet — der Client lädt es
 * trotz Häkchen nicht, solange „Veraltete AddOns laden" aus ist.
 */
const activeButOutdated = computed(
  () =>
    props.scan.addons.filter(
      (addon) =>
        stateFor(props.character, addon) === true &&
        interfaceStatus(addon, props.clientInterface) === "outdated",
    ).length,
);
</script>

<template>
  <section>
    <h2 class="tome-heading mb-2">{{ character.label }}</h2>

    <p class="tome-data mb-4 text-sm opacity-70">
      {{ t("character.tally", { on: tally.on, off: tally.off, unseen: tally.unseen }) }}
    </p>

    <p
      v-if="activeButOutdated"
      class="text-verdict-warn dark:text-verdict-warn-dark mb-4 text-sm"
    >
      {{ t("character.activeButOutdated", { n: activeButOutdated }, activeButOutdated) }}
    </p>

    <AddonTable :scan="scan" :client-interface="clientInterface" :character="character" />
  </section>
</template>
