# Tome of Addons

*The curated tome of WoW 1.12.1 addons.*

Cross-platform Desktop-App für WoW 1.12.1 Addon-Management.

- Repo: `vanilla-wow-dev/tome-of-addons`
- Landing: `vanilla-wow.dev`
- App-Identifier: `dev.vanilla-wow.tome-of-addons`
- Konzept: [`docs/concept.md`](docs/concept.md) — Architektur, Identitäts-Modell,
  Trust-Modell, Phasen-Plan, Entscheidungs-Log.

Aktueller Stand: **v0.2.1**. Self-Update (MVP-3) läuft End-to-End, die
WoW-Installations-Erkennung steht. Die Addon-Logik (Scanner, Tree-Hash,
`.toc`-Parser) ist noch nicht implementiert.

## Stack

- Tauri v2 (Rust-Backend, Vue 3 + Vite + TypeScript frontend)
- `tauri-plugin-updater` (Ed25519-signiert)
- `tauri-plugin-process` (Restart-after-Update)
- CI: GitHub Actions + `tauri-action` (Cross-Build für macOS/Linux/Windows)

## Lokale Entwicklung

```bash
npm install
npm run tauri dev      # startet Dev-App
npm run tauri build    # baut Release-Binary in src-tauri/target/release/bundle/
```

Linux-Dependencies (Arch): `webkit2gtk-4.1`, `libsoup-3.0`, `javascriptcoregtk-4.1`.

## Self-Update einrichten

### 1. GitHub-Repo anlegen

Aktuell zeigt `tauri.conf.json` auf `https://github.com/vanilla-wow-dev/tome-of-addons`.
Wenn Org-Slug oder Repo-Name abweichen, das Feld `plugins.updater.endpoints` anpassen.

### 2. Signing-Key

Beim Prototyp-Aufbau wurde ein Ed25519-Keypair erzeugt:

- Privater Schlüssel: `~/.tauri/tome-of-addons.key` (NICHT commiten)
- Öffentlicher Schlüssel: `~/.tauri/tome-of-addons.key.pub`
- Pubkey ist bereits in `src-tauri/tauri.conf.json` unter `plugins.updater.pubkey`
  eingebettet.

**Achtung:** Der Schlüssel wurde **ohne Passwort** erzeugt. Vor erstem
Production-Release rotieren mit:

```bash
npx tauri signer generate -w ~/.tauri/tome-of-addons.key -p "<starkes-passwort>" -f
```

und neuen Pubkey in `tauri.conf.json` eintragen.

### 3. it4c-release-bot installieren

release-please läuft unter dem `it4c-release-bot` GitHub-App-Account (gleiche
App wie auf jahrweiser, sauberere Attribution als `github-actions[bot]`).

1. GitHub-App `it4c-release-bot` auf der Org `vanilla-wow-dev` oder direkt
   auf `vanilla-wow-dev/tome-of-addons` installieren.
2. App-Permissions müssen `contents: write` und `pull-requests: write`
   enthalten — bei jahrweiser bereits so konfiguriert.

### 4. Repo-Variablen und Secrets setzen

Unter Settings → Secrets and variables → Actions:

**Variables:**

| Variable | Wert |
| --- | --- |
| `RELEASE_APP_ID` | App-ID des `it4c-release-bot` (gleicher Wert wie auf jahrweiser) |

**Secrets:**

| Secret | Wert |
| --- | --- |
| `RELEASE_APP_PRIVATE_KEY` | Private Key des `it4c-release-bot` (PEM, gleicher Wert wie auf jahrweiser) |
| `TAURI_SIGNING_PRIVATE_KEY` | Inhalt von `~/.tauri/tome-of-addons.key` (komplette Datei) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Passwort des Keys, leer wenn ohne Passwort erzeugt |

**Tipp:** Wenn `RELEASE_APP_ID` und `RELEASE_APP_PRIVATE_KEY` auf Org-Level
(`vanilla-wow-dev`) gesetzt sind, erben alle Repos in der Org sie automatisch
— einmaliges Setup für künftige Tools.

### 5. Release-Flow (release-please)

Versionen werden **nicht manuell gebumpt**. Stattdessen orchestriert
[release-please](https://github.com/googleapis/release-please) den ganzen
Release-Vorgang basierend auf
[Conventional Commits](https://www.conventionalcommits.org/).

**Workflow:**

1. Auf `master` committen mit konventionellem Format:
   ```
   feat(updater): add manual check button     # bumpt minor
   fix(client): handle null guild correctly   # bumpt patch
   feat(client)!: rename addon-id schema      # bumpt major
   ```
2. Push auf `master` → release-please öffnet (oder updated) eine
   **Release-PR** mit:
   - Bumps in `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`
   - Generierter `CHANGELOG.md`-Eintrag
3. PR reviewen, mergen → release-please:
   - Erstellt Git-Tag `v0.1.1`
   - Erstellt GitHub-Release mit Changelog
4. Im selben Workflow-Run startet danach `build-tauri` (matrix: macOS arm64+x86_64,
   Ubuntu, Windows) und hängt die signierten Binaries + `latest.json` an
   das eben erstellte Release.

Initiales `v0.1.0` Release: einfach den ersten konventionellen Commit auf master
pushen — release-please erkennt das fehlende Tag und schlägt v0.1.0 als
ersten Release vor.

**PR-Title-Lint:** `pr-title-lint.yml` erzwingt Conventional-Commits-Format
auf PR-Titeln (Squash-Merge übernimmt den PR-Titel als Commit-Message, daher
genügt das für release-please).

Erlaubte Scopes: `client`, `tauri`, `updater`, `ui`, `ci`, `docs`, `deps`,
`release`. Scope ist optional (release-please erzeugt `chore(release): …`
automatisch).

### 6. Update testen

1. v0.1.0 lokal installieren (Bundle aus dem Release).
2. App starten → zeigt v0.1.0, „Auf Updates prüfen" → up-to-date.
3. Einen `fix`- oder `feat`-Commit auf master pushen.
4. Release-PR von release-please mergen → v0.1.1 wird automatisch getaggt
   und gebaut.
5. Installierte v0.1.0 starten → „Auf Updates prüfen" → Banner „v0.1.1
   verfügbar" → Download → Restart → läuft als v0.1.1.

## Verzeichnis-Layout

```
tome-of-addons/                # Repo-Slug auf GitHub
├── docs/
│   └── concept.md             # Architektur- und Konzept-Referenz
├── crates/                    # Tauri-freie Kern-Crates, gemeinsam mit dem Collector
│   ├── tree-hash/             # toa-tree-v1: kanonischer Identitäts-Hash
│   └── toc/                   # .toc-Parser
├── src/                       # Vue-Frontend
│   ├── App.vue                # Version, WoW-Erkennung, Update-Flow
│   ├── RootCard.vue           # Darstellung einer WoW-Installation
│   ├── i18n.ts, locales/      # de / en / fr
│   └── wow.ts                 # Typen + Formatierung
├── src-tauri/
│   ├── src/lib.rs             # Tauri-Builder, Plugins registriert
│   ├── src/commands.rs        # IPC-Wrapper
│   ├── src/addons.rs          # Scanner für Interface/AddOns + Hash-Cache
│   ├── src/wow.rs             # WoW-Root-Erkennung (Walk-up + Registry)
│   ├── src/exe.rs             # WoW.exe-Analyse (Version/Build)
│   ├── src/relocate.rs        # Manager in WoW-Ordner verschieben
│   ├── tauri.conf.json        # Updater-Endpoint + Pubkey
│   ├── capabilities/default.json
│   └── Cargo.toml
├── .github/workflows/
│   ├── ci.yml                # typecheck, clippy, fmt, 100%-Coverage-Gates
│   ├── release.yml           # release-please + tauri-action matrix build
│   └── pr-title-lint.yml     # conventional commit enforcement
├── release-please-config.json
├── .release-please-manifest.json
└── package.json
```

## Nächste Schritte

MVP-0 fertigstellen:

1. ✅ **`crates/tree-hash`** — kanonischer Tree-Hash `toa-tree-v1` (SHA-256 über
   den normalisierten Datei-Baum).
2. ✅ **`crates/toc`** — `.toc`-Parser.
3. ✅ **`src-tauri/src/addons.rs`** — Walk über `Interface/AddOns/`,
   Mode-Detection, Hash-Cache, `rayon`.
4. ⬜ **UI-Fundament** — Tailwind + TanStack Table einziehen, bestehende Views
   migrieren.
5. ⬜ **Addon-Liste** — Name, Version, Tree-Hash, Mode, Größe.

Danach MVP-1 (Direct-Git-Install), MVP-2 (Index-Subscription), MVP-4 (Indexed
Install). Vollbild und Begründungen in [`docs/concept.md`](docs/concept.md).
