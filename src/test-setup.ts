// Stellt im Test-Environment ein in-memory localStorage bereit. jsdom liefert
// ohne http-Origin keinen Web-Storage, und Node 22+ überschattet das globale
// `localStorage`. Die App nutzt window.localStorage — hier wird es polyfilled.
if (!window.localStorage) {
  const store = new Map<string, string>();
  const mock = {
    get length() {
      return store.size;
    },
    clear: () => store.clear(),
    getItem: (key: string) => (store.has(key) ? store.get(key)! : null),
    key: (i: number) => [...store.keys()][i] ?? null,
    removeItem: (key: string) => void store.delete(key),
    setItem: (key: string, value: string) => void store.set(key, String(value)),
  } as Storage;
  Object.defineProperty(window, "localStorage", { value: mock, configurable: true });
}
