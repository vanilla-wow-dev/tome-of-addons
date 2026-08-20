<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  createColumnHelper,
  getCoreRowModel,
  getExpandedRowModel,
  getSortedRowModel,
  useVueTable,
  type ExpandedState,
  type SortingState,
} from "@tanstack/vue-table";
import {
  compareTitles,
  countOutdated,
  fmtCount,
  interfaceStatus,
  matchesQuery,
  shortHash,
  type Addon,
  type AddonScan,
  type Character,
} from "./addons";
import { fmtBytes } from "./wow";

const props = defineProps<{
  scan: AddonScan;
  characters: Character[];
  /** Interface-Version des Clients, Bezugswert für die Veraltet-Erkennung. */
  clientInterface: string | null;
}>();
/** Pfad zur AddOns.txt des gewählten Charakters, `null` = keiner gewählt. */
const character = defineModel<string | null>("character", { required: true });
const { t } = useI18n();

const query = ref("");
const developerOnly = ref(false);
const outdatedOnly = ref(false);
const sorting = ref<SortingState>([{ id: "title", desc: false }]);
const expanded = ref<ExpandedState>({});
const showSkipped = ref(false);
const copied = ref<string | null>(null);

/**
 * Filtern passiert hier statt über TanStack: `matchesQuery` ist eine reine,
 * separat getestete Funktion, und die Filterregel (Hash und Pfad werden
 * mitdurchsucht) ist fachliche Logik, die nicht in einer Tabellenkonfiguration
 * versteckt werden sollte.
 */
const rows = computed(() =>
  props.scan.addons.filter(
    (addon) =>
      (!developerOnly.value || addon.mode === "developer") &&
      (!outdatedOnly.value || interfaceStatus(addon, props.clientInterface) === "outdated") &&
      matchesQuery(addon, query.value),
  ),
);

const outdatedCount = computed(() => countOutdated(props.scan.addons, props.clientInterface));

const columnHelper = createColumnHelper<Addon>();
const columns = [
  columnHelper.accessor("title", {
    id: "title",
    sortingFn: (a, b) => compareTitles(a.original, b.original),
  }),
  columnHelper.accessor((addon) => addon.version ?? "", { id: "version" }),
  columnHelper.accessor((addon) => addon.interface ?? "", { id: "interface" }),
  columnHelper.accessor((addon) => addon.tree_sha_short ?? "", { id: "hash" }),
  // Unbekannt sortiert zwischen aktiv und inaktiv, statt mit einem der beiden
  // zu verschmelzen — „nie gesehen" ist etwas anderes als „abgeschaltet".
  columnHelper.accessor((addon) => (addon.enabled === null ? 1 : addon.enabled ? 2 : 0), {
    id: "enabled",
  }),
  columnHelper.accessor("mode", { id: "mode" }),
  columnHelper.accessor("file_count", { id: "files" }),
  columnHelper.accessor("size_bytes", { id: "size" }),
];

/** Spalten, deren Werte Zahlen sind, werden rechtsbündig gesetzt. */
const NUMERIC = new Set(["files", "size"]);

const table = useVueTable({
  get data() {
    return rows.value;
  },
  columns,
  state: {
    get sorting() {
      return sorting.value;
    },
    get expanded() {
      return expanded.value;
    },
  },
  onSortingChange: (updater) => {
    sorting.value = typeof updater === "function" ? updater(sorting.value) : updater;
  },
  onExpandedChange: (updater) => {
    expanded.value = typeof updater === "function" ? updater(expanded.value) : updater;
  },
  getRowId: (addon) => addon.id,
  getCoreRowModel: getCoreRowModel(),
  getSortedRowModel: getSortedRowModel(),
  getExpandedRowModel: getExpandedRowModel(),
});

/**
 * Kopiert den vollen Tree-Hash. Die Clipboard-API fehlt in unsicheren Kontexten
 * und in Testumgebungen — dann passiert nichts weiter, statt zu werfen.
 */
async function copyHash(addon: Addon) {
  // Kein Optional-Chaining auf writeText: fehlt die API, liefe der Aufruf
  // stillschweigend durch und wir meldeten „Kopiert", ohne kopiert zu haben.
  if (!addon.tree_sha || !navigator.clipboard) return;
  try {
    await navigator.clipboard.writeText(addon.tree_sha);
    copied.value = addon.id;
  } catch {
    copied.value = null;
  }
}
</script>

<template>
  <section class="tome-panel mt-8 p-5">
    <h2 class="tome-heading mb-4">{{ t("addons.title") }}</h2>

    <div class="mb-4 flex flex-wrap items-center gap-4">
      <input
        v-model="query"
        type="search"
        class="tome-input min-w-56 flex-1"
        :placeholder="t('addons.search')"
        :aria-label="t('addons.search')"
      />
      <label v-if="props.characters.length" class="flex items-center gap-2 text-sm">
        <span class="opacity-70">{{ t("addons.character") }}</span>
        <select v-model="character" class="tome-input text-sm">
          <option :value="null">{{ t("addons.noCharacter") }}</option>
          <option v-for="c in props.characters" :key="c.path" :value="c.path">
            {{ c.label }}
          </option>
        </select>
      </label>
      <label class="flex cursor-pointer items-center gap-2 text-sm">
        <input v-model="developerOnly" type="checkbox" class="accent-gold-700" />
        {{ t("addons.developerOnly") }}
      </label>
      <label
        v-if="outdatedCount"
        class="text-verdict-warn dark:text-verdict-warn-dark flex cursor-pointer items-center gap-2 text-sm"
      >
        <input v-model="outdatedOnly" type="checkbox" class="accent-gold-700" />
        {{ t("addons.outdatedOnly", { n: outdatedCount }) }}
      </label>
      <p class="tome-data ml-auto opacity-70">
        {{ t("addons.count", { shown: rows.length, total: props.scan.addons.length }) }}
      </p>
    </div>

    <!-- Der Client lädt veraltete Addons nur mit dem entsprechenden Haken im
         AddOn-Fenster. Ohne diesen Hinweis sucht der Nutzer den Fehler beim
         Addon statt bei der Interface-Version. -->
    <p
      v-if="outdatedCount"
      class="text-verdict-warn dark:text-verdict-warn-dark mb-4 text-sm"
    >
      {{ t("addons.outdatedHint", { n: outdatedCount, client: props.clientInterface }, outdatedCount) }}
    </p>

    <div class="overflow-x-auto">
      <table class="w-full border-collapse text-left">
        <thead>
          <tr class="border-b border-gold-700/50">
            <th
              v-for="header in table.getHeaderGroups()[0].headers"
              :key="header.id"
              scope="col"
              :aria-sort="
                header.column.getIsSorted() === 'asc'
                  ? 'ascending'
                  : header.column.getIsSorted() === 'desc'
                    ? 'descending'
                    : 'none'
              "
              :class="['py-2 pr-3', NUMERIC.has(header.column.id) ? 'text-right' : '']"
            >
              <button
                type="button"
                class="font-display cursor-pointer text-[0.7rem] font-bold tracking-[0.14em] text-gold-700 uppercase hover:underline dark:text-gold-300"
                @click="header.column.getToggleSortingHandler()?.($event)"
              >
                {{ t(`addons.columns.${header.column.id}`) }}
                <!-- Der Platzhalter bleibt stehen, damit die Kopfzeile beim
                     Sortieren nicht springt — aber gedimmt, sonst rauscht ein
                     Zeichen hinter jeder der sechs Spalten. -->
                <span
                  aria-hidden="true"
                  :class="header.column.getIsSorted() ? '' : 'opacity-25'"
                  >{{
                    header.column.getIsSorted() === "asc"
                      ? "▲"
                      : header.column.getIsSorted() === "desc"
                        ? "▼"
                        : "▲"
                  }}</span
                >
              </button>
            </th>
          </tr>
        </thead>

        <tbody>
          <template v-for="row in table.getRowModel().rows" :key="row.id">
            <tr class="border-b border-ink-500/15 align-top">
              <td class="py-1.5 pr-3">
                <button
                  type="button"
                  class="flex w-full cursor-pointer items-baseline gap-2 text-left"
                  :aria-expanded="row.getIsExpanded()"
                  @click="row.toggleExpanded()"
                >
                  <span class="text-gold-700 dark:text-gold-300" aria-hidden="true">{{
                    row.getIsExpanded() ? "▼" : "▸"
                  }}</span>
                  <span class="font-semibold">{{ row.original.title }}</span>
                  <span
                    v-if="row.original.error"
                    class="tome-badge text-verdict-bad dark:text-verdict-bad-dark"
                    >{{ t("addons.broken") }}</span
                  >
                </button>
              </td>
              <td class="tome-data py-1.5 pr-3">{{ row.original.version ?? "—" }}</td>
              <td class="py-1.5 pr-3">
                <span
                  v-if="row.original.interface"
                  :class="[
                    'tome-data',
                    interfaceStatus(row.original, props.clientInterface) === 'outdated'
                      ? 'text-verdict-warn dark:text-verdict-warn-dark font-semibold'
                      : '',
                  ]"
                  :title="
                    interfaceStatus(row.original, props.clientInterface) === 'outdated'
                      ? t('addons.outdatedTitle', { client: props.clientInterface })
                      : ''
                  "
                  >{{ row.original.interface
                  }}<span
                    v-if="interfaceStatus(row.original, props.clientInterface) === 'outdated'"
                    class="ml-1"
                    >⚠</span
                  ></span
                >
                <span v-else class="tome-data opacity-50">—</span>
              </td>
              <td class="tome-data py-1.5 pr-3">{{ shortHash(row.original) }}</td>
              <td class="py-1.5 pr-3">
                <span
                  v-if="row.original.enabled === true"
                  class="tome-badge text-verdict-ok dark:text-verdict-ok-dark"
                  >{{ t("addons.enabled") }}</span
                >
                <span
                  v-else-if="row.original.enabled === false"
                  class="tome-badge opacity-50"
                  >{{ t("addons.disabled") }}</span
                >
                <span
                  v-else
                  class="tome-data opacity-40"
                  :title="t('addons.unseenTitle')"
                  >—</span
                >
              </td>
              <td class="py-1.5 pr-3">
                <!-- Nur die Ausnahme benennen: „Git-Checkout". Ob die übrigen
                     Dateien aus einem ZIP, von Hand oder von einem anderen
                     Manager stammen, wissen wir nicht — „ZIP" wäre erfunden. -->
                <span
                  v-if="row.original.mode === 'developer'"
                  class="tome-badge text-verdict-warn dark:text-verdict-warn-dark"
                  >{{ t("addons.mode.developer") }}</span
                >
              </td>
              <td class="tome-data py-1.5 pr-3 text-right">
                {{ fmtCount(row.original.file_count) }}
              </td>
              <td class="tome-data py-1.5 text-right">{{ fmtBytes(row.original.size_bytes) }}</td>
            </tr>

            <tr v-if="row.getIsExpanded()" class="border-b border-ink-500/15">
              <td colspan="8" class="bg-gold-500/5 px-6 py-3">
                <dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
                  <dt class="opacity-60">{{ t("addons.detail.author") }}</dt>
                  <dd>{{ row.original.author ?? "—" }}</dd>

                  <dt class="opacity-60">{{ t("addons.detail.interface") }}</dt>
                  <dd class="tome-data">
                    {{ row.original.interface ?? "—" }}
                    <span
                      v-if="interfaceStatus(row.original, props.clientInterface) === 'outdated'"
                      class="text-verdict-warn dark:text-verdict-warn-dark"
                      >{{ t("addons.outdatedTitle", { client: props.clientInterface }) }}</span
                    >
                  </dd>

                  <dt class="opacity-60">{{ t("addons.detail.defaultState") }}</dt>
                  <dd class="tome-data">{{ row.original.default_state ?? "—" }}</dd>

                  <dt class="opacity-60">{{ t("addons.detail.notes") }}</dt>
                  <dd>{{ row.original.notes ?? "—" }}</dd>

                  <dt class="opacity-60">{{ t("addons.detail.path") }}</dt>
                  <dd class="tome-data break-all">{{ row.original.path }}</dd>

                  <dt class="opacity-60">{{ t("addons.detail.hash") }}</dt>
                  <dd class="tome-data flex flex-wrap items-center gap-2 break-all">
                    <span>{{ row.original.tree_sha ?? "—" }}</span>
                    <button
                      v-if="row.original.tree_sha"
                      type="button"
                      class="tome-badge cursor-pointer"
                      @click="copyHash(row.original)"
                    >
                      {{ copied === row.original.id ? t("addons.copied") : t("addons.copy") }}
                    </button>
                    <span v-if="row.original.cached" class="tome-badge opacity-60">{{
                      t("addons.cached")
                    }}</span>
                  </dd>

                  <template v-if="row.original.error">
                    <dt class="opacity-60">{{ t("addons.detail.error") }}</dt>
                    <dd class="text-verdict-bad dark:text-verdict-bad-dark">
                      {{ row.original.error }}
                    </dd>
                  </template>
                </dl>
              </td>
            </tr>
          </template>

          <tr v-if="rows.length === 0">
            <td colspan="8" class="py-6 text-center opacity-70">{{ t("addons.empty") }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Übersprungene Ordner bleiben sichtbar statt still zu verschwinden:
         ein falsch benanntes .toc ist ein echter Installationsfehler, den der
         Nutzer sonst nie erfährt (WoW lädt den Ordner ebenfalls nicht). -->
    <div v-if="props.scan.skipped.length" class="mt-5 border-t border-gold-700/30 pt-3">
      <button
        type="button"
        class="cursor-pointer text-sm opacity-70 hover:opacity-100"
        :aria-expanded="showSkipped"
        @click="showSkipped = !showSkipped"
      >
        <span aria-hidden="true">{{ showSkipped ? "▼" : "▸" }}</span>
        {{ t("addons.skipped", { n: props.scan.skipped.length }, props.scan.skipped.length) }}
      </button>
      <ul v-if="showSkipped" class="mt-2 space-y-1 text-sm">
        <li v-for="folder in props.scan.skipped" :key="folder.id" class="flex flex-wrap gap-2">
          <span class="tome-data">{{ folder.id }}</span>
          <span class="opacity-60">{{ folder.reason }}</span>
        </li>
      </ul>
    </div>
  </section>
</template>
