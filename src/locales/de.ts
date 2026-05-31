// Deutsch.
export default {
  tagline: "Das kuratierte Verzeichnis der WoW-1.12.1-Addons.",
  language: "Sprache",
  wow: {
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
  update: {
    available: "Version {version} verfügbar (aktuell {current}).",
    download: "Update herunterladen & installieren",
    downloading: "Lade…",
    ready: "Update installiert. Neustart erforderlich.",
    restart: "Jetzt neustarten",
    downloadFailed: "Download fehlgeschlagen: {err}",
  },
} as const;
