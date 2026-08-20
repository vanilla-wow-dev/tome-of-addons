// Deutsch.
export default {
  tagline: "Das kuratierte Verzeichnis der WoW-1.12.1-Addons.",
  language: "Sprache",
  nav: {
    label: "Bereiche",
    wow: "WoW-Zustand",
    addons: "Addons",
    characters: "Charaktere",
    health: { ok: "Alles in Ordnung", warn: "Mit Einschränkungen", error: "Keine Installation" },
  },
  health: {
    notManaged:
      "Tome of Addons liegt nicht in einem WoW-Ordner und verwaltet daher keine Installation.",
    clientRunningHere:
      "WoW läuft gerade. Änderungen an SavedVariables werden beim Beenden überschrieben, und neu installierte Addons erkennt der Client erst nach einem Neustart.",
    clientRunningUnknown:
      "Ein WoW-Prozess läuft, ließ sich aber keiner Installation zuordnen. Falls es diese ist: Änderungen können verlorengehen.",
    noAddonsFolder: "Kein Interface/AddOns-Ordner — es kann noch nichts installiert sein.",
    exe: {
      modified: "Die WoW.exe ist gepatcht (weicht vom offiziellen Hash ab).",
      "unknown-build": "Unbekannter Build — kein offizieller Referenz-Hash vorhanden.",
      unknown: "Kein erkennbarer WoW-Client.",
    },
  },
  character: {
    tally: "{on} aktiv · {off} aus · {unseen} nie gesehen",
    activeButOutdated:
      "{n} aktives Addon ist veraltet und wird trotz Häkchen nicht geladen. | {n} aktive Addons sind veraltet und werden trotz Häkchen nicht geladen.",
  },
  wow: {
    searching: "Suche WoW-Installation…",
    managed: "Verwaltet",
    others: "Weitere erkannte Installationen (nicht verwaltet)",
    moveHint: "Um eine andere zu verwalten, verschiebe den Manager dorthin.",
    moveHere: "Manager hierher verschieben",
    unanchored:
      "Tome of Addons liegt nicht in einem WoW-Ordner und verwaltet daher noch keine Installation. Erkannt wurde:",
    none: "Keine WoW-1.12.1-Installation gefunden. Lege den Manager in deinen WoW-Ordner und starte ihn dort.",
    searchFailed: "WoW-Suche fehlgeschlagen: {err}",
  },
  relocate: {
    inProgress: "Verschiebe…",
    done: "Manager nach {dest} kopiert und dort gestartet. Diese Instanz wird geschlossen…",
    failed: "Verschieben fehlgeschlagen: {err}",
  },
  exe: {
    build: "Build",
    size: "Größe",
    identity: {
      official: "✓ Offiziell {version} ({locale})",
      modified: "⚠ Modifiziert (gibt sich als {version} aus)",
      unknownBuild: "⚠ Unbekannter Build (kein offizieller Referenz-Hash)",
      unknown: "✗ Kein erkennbarer WoW-Client",
    },
  },
  addons: {
    title: "Addons",
    search: "Suchen (Name, Autor, Hash, Pfad)…",
    developerOnly: "Nur Git-Checkouts",
    outdatedOnly: "Nur veraltete ({n})",
    outdatedHint:
      "{n} Addon hat eine andere Interface-Version als dein Client ({client}). WoW lädt es nur, wenn im AddOn-Fenster „Veraltete AddOns laden“ aktiviert ist. | {n} Addons haben eine andere Interface-Version als dein Client ({client}). WoW lädt sie nur, wenn im AddOn-Fenster „Veraltete AddOns laden“ aktiviert ist.",
    outdatedTitle: "Veraltet — Client erwartet {client}",
    character: "Charakter",
    noCharacter: "— keiner —",
    enabled: "aktiv",
    disabled: "aus",
    unseenTitle: "Dem Client noch nie begegnet",
    scanProgress: "Prüfe Addons… {done} von {total}",
    count: "{shown} von {total}",
    empty: "Keine Addons passen zur Suche.",
    broken: "Fehler",
    cached: "aus Cache",
    copy: "Kopieren",
    copied: "Kopiert",
    skipped: "{n} Ordner übersprungen (kein passendes .toc) | {n} Ordner übersprungen (kein passendes .toc)",
    scanFailed: "Addon-Scan fehlgeschlagen: {err}",
    scanning: "Durchsuche Addons…",
    columns: {
      title: "Addon",
      version: "Version",
      interface: "Interface",
      hash: "Hash",
      enabled: "Aktiv",
      mode: "Modus",
      files: "Dateien",
      size: "Größe",
    },
    mode: { consumer: "", developer: "Git" },
    detail: {
      author: "Autor",
      interface: "Interface",
      defaultState: "DefaultState (.toc)",
      notes: "Notizen",
      path: "Pfad",
      hash: "Tree-Hash",
      error: "Fehler",
    },
  },
  update: {
    available: "Version {version} verfügbar (aktuell {current}).",
    download: "Update herunterladen & installieren",
    downloading: "Lade…",
    ready: "Update installiert. Neustart erforderlich.",
    restart: "Jetzt neustarten",
    downloadFailed: "Download fehlgeschlagen: {err}",
  },
} as const;
