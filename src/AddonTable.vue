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
  fmtCount,
  matchesQuery,
  shortHash,
  type Addon,
  type AddonScan,
} from "./addons";
import { fmtBytes } from "./wow";

const props = defineProps<{ scan: AddonScan }>();
const { t } = useI18n();

const query = ref("");
const developerOnly = ref(false);
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
      (!developerOnly.value || addon.mode === "developer") && matchesQuery(addon, query.value),
  ),
);

const columnHelper = createColumnHelper<Addon>();
const columns = [
  columnHelper.accessor("title", {
    id: "title",
    sortingFn: (a, b) => compareTitles(a.original, b.original),
  }),
  columnHelper.accessor((addon) => addon.version ?? "", { id: "version" }),
  columnHelper.accessor((addon) => addon.tree_sha_short ?? "", { id: "hash" }),
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
      <label class="flex cursor-pointer items-center gap-2 text-sm">
        <input v-model="developerOnly" type="checkbox" class="accent-gold-700" />
        {{ t("addons.developerOnly") }}
      </label>
      <p class="tome-data ml-auto opacity-70">
        {{ t("addons.count", { shown: rows.length, total: props.scan.addons.length }) }}
      </p>
    </div>

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
              <td class="tome-data py-1.5 pr-3">{{ shortHash(row.original) }}</td>
              <td class="py-1.5 pr-3">
                <span
                  :class="[
                    'tome-badge',
                    row.original.mode === 'developer'
                      ? 'text-verdict-warn dark:text-verdict-warn-dark'
                      : 'opacity-70',
                  ]"
                  >{{ t(`addons.mode.${row.original.mode}`) }}</span
                >
              </td>
              <td class="tome-data py-1.5 pr-3 text-right">
                {{ fmtCount(row.original.file_count) }}
              </td>
              <td class="tome-data py-1.5 text-right">{{ fmtBytes(row.original.size_bytes) }}</td>
            </tr>

            <tr v-if="row.getIsExpanded()" class="border-b border-ink-500/15">
              <td colspan="6" class="bg-gold-500/5 px-6 py-3">
                <dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
                  <dt class="opacity-60">{{ t("addons.detail.author") }}</dt>
                  <dd>{{ row.original.author ?? "—" }}</dd>

                  <dt class="opacity-60">{{ t("addons.detail.interface") }}</dt>
                  <dd class="tome-data">{{ row.original.interface ?? "—" }}</dd>

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
            <td colspan="6" class="py-6 text-center opacity-70">{{ t("addons.empty") }}</td>
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
        {{ t("addons.skipped", { n: props.scan.skipped.length }) }}
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
