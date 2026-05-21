# Tome of Addons

*The curated tome of WoW 1.12.1 addons.*

Cross-platform Desktop-App für WoW 1.12.1 Addon-Management.

- Repo: `vanilla-wow-dev/tome-of-addons`
- Landing: `vanilla-wow.dev`
- App-Identifier: `dev.vanilla-wow.tome-of-addons`
- Konzept: `~/.data/sources/ulfgebhardt/concept/projects/wow-addon-manager.md`
  (Working-Title aus der frühen Konzept-Phase — finaler Produkt-Name ist Tome of Addons.)

Aktueller Stand: **MVP-3-Prototyp (Self-Update End-to-End)**. UI zeigt nur
Version und Update-Banner. Addon-Logik ist noch nicht implementiert.

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

### 3. GitHub-Secrets setzen

Im GitHub-Repo unter Settings → Secrets and variables → Actions:

| Secret | Wert |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | Inhalt von `~/.tauri/tome-of-addons.key` (komplette Datei) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Passwort des Keys, leer wenn ohne Passwort erzeugt |

### 4. Erstes Release

```bash
git add .
git commit -m "Initial prototype"
git tag v0.1.0
git push origin master --tags
```

Der `Release`-Workflow läuft auf macOS, Linux, Windows parallel und
veröffentlicht ein GitHub-Release mit:
- Installer-Bundles (`.dmg`, `.AppImage`, `.deb`, `.msi`, `.exe`)
- Signature-Dateien (`*.sig`)
- `latest.json` — vom Updater-Plugin gepollt

### 5. Update testen

1. v0.1.0 lokal installieren (Bundle aus dem Release).
2. App starten → zeigt v0.1.0, „Auf Updates prüfen" → up-to-date.
3. `version` in `src-tauri/tauri.conf.json` und `package.json` auf `0.1.1` bumpen.
4. `git tag v0.1.1 && git push --tags` → Release wird automatisch gebaut.
5. Installierte v0.1.0 starten → „Auf Updates prüfen" → Banner „v0.1.1
   verfügbar" → Download → Restart → läuft als v0.1.1.

## Verzeichnis-Layout

```
tome-of-addons/                # Repo-Slug auf GitHub
├── src/                       # Vue-Frontend
│   └── App.vue                # Minimal-UI: Version + Update-Flow
├── src-tauri/
│   ├── src/lib.rs             # Tauri-Builder, Plugins registriert
│   ├── tauri.conf.json        # Updater-Endpoint + Pubkey
│   ├── capabilities/default.json
│   └── Cargo.toml
├── .github/workflows/release.yml
└── package.json
```

## Nächste MVPs (laut Konzept)

- **MVP-0**: Addon-Scanner (`Interface/AddOns/`-Walk, Tree-SHA, mtime-Cache)
- **MVP-1**: Direct-Git-Install
- **MVP-2**: Index-Subscription (Mojotrollz-Index)
- **MVP-4**: Indexed Install + Consumer/Developer-Mode

Siehe Konzept-Dokument für Vollbild.
