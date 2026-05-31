// English — base/source locale. de.ts and fr.ts must mirror these keys.
export default {
  tagline: "The curated tome of WoW 1.12.1 addons.",
  language: "Language",
  wow: {
    managed: "Managed",
    others: "Other detected installations (not managed)",
    moveHint: "To manage another one, move the manager there.",
    moveHere: "Move manager here",
    unanchored:
      "Tome of Addons is not located in a WoW folder and therefore does not manage any installation yet. Detected:",
    none: "No WoW 1.12.1 installation found. Place the manager in your WoW folder and start it there.",
    searchFailed: "WoW search failed: {err}",
  },
  relocate: {
    inProgress: "Moving…",
    done: "Manager copied to {dest} and started there. This instance will close…",
    failed: "Move failed: {err}",
  },
  exe: {
    build: "Build",
    size: "Size",
    identity: {
      official: "✓ Official {version} ({locale})",
      modified: "⚠ Modified (claims to be {version})",
      unknownBuild: "⚠ Unknown build (no official reference hash)",
      unknown: "✗ Not a recognizable WoW client",
    },
  },
  update: {
    available: "Version {version} available (current {current}).",
    download: "Download & install update",
    downloading: "Downloading…",
    ready: "Update installed. Restart required.",
    restart: "Restart now",
    downloadFailed: "Download failed: {err}",
  },
} as const;
