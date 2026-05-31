// Français.
export default {
  tagline: "Le répertoire organisé des addons WoW 1.12.1.",
  language: "Langue",
  wow: {
    managed: "Géré",
    others: "Autres installations détectées (non gérées)",
    moveHint: "Pour en gérer une autre, déplacez-y le gestionnaire.",
    moveHere: "Déplacer le gestionnaire ici",
    unanchored:
      "Tome of Addons ne se trouve pas dans un dossier WoW et ne gère donc encore aucune installation. Détecté :",
    none: "Aucune installation WoW 1.12.1 trouvée. Placez le gestionnaire dans votre dossier WoW et lancez-le depuis là.",
    searchFailed: "Échec de la recherche WoW : {err}",
  },
  relocate: {
    inProgress: "Déplacement…",
    done: "Gestionnaire copié vers {dest} et lancé depuis là. Cette instance va se fermer…",
    failed: "Échec du déplacement : {err}",
  },
  exe: {
    build: "Build",
    size: "Taille",
    identity: {
      official: "✓ Officiel {version} ({locale})",
      modified: "⚠ Modifié (se présente comme {version})",
      unknownBuild: "⚠ Build inconnu (aucun hash de référence officiel)",
      unknown: "✗ Client WoW non reconnu",
    },
  },
  update: {
    available: "Version {version} disponible (actuelle {current}).",
    download: "Télécharger et installer la mise à jour",
    downloading: "Téléchargement…",
    ready: "Mise à jour installée. Redémarrage requis.",
    restart: "Redémarrer maintenant",
    downloadFailed: "Échec du téléchargement : {err}",
  },
} as const;
