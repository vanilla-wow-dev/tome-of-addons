# Tome of Addons — Konzept

*The curated tome of WoW 1.12.1 addons.*

Dieses Dokument ist die inhaltliche Referenz für die Umsetzung. Für Build,
Release-Workflow und lokale Entwicklung siehe [`../README.md`](../README.md).

- Repo: `vanilla-wow-dev/tome-of-addons`
- Landing: `vanilla-wow.dev`
- App-Identifier: `dev.vanilla-wow.tome-of-addons`

Das Konzept entstand ursprünglich unter dem Arbeitstitel `mojotrollz/addon-manager`
in der Mojotrollz-Community. Produkt und Infrastruktur sind seither bewusst
community-neutral aufgestellt (siehe [Entscheidungs-Log](#entscheidungs-log), E-4).

---

## Stand der Umsetzung

Aktuell veröffentlicht: **v0.2.1**.

| Bereich | Stand |
| --- | --- |
| Tauri-v2-Skelett, Vue 3 + Vite + TypeScript | ✅ |
| MVP-3 — Self-Update | ✅ End-to-End (Ed25519, Auto-Check bei Start + alle 24 h, Progress, Restart-Banner) |
| Release-Pipeline | ✅ release-please + `it4c-release-bot` + `tauri-action` (macOS arm64/x86_64, Ubuntu, Windows) |
| Qualitätsgate | ✅ `vue-tsc`, `clippy -D warnings`, `rustfmt`, 100-%-Coverage-Gate (Rust via `cargo-llvm-cov`, Frontend via Vitest) |
| Lokalisierung | ✅ de / en / fr (`vue-i18n`) |
| MVP-0 — WoW-Erkennung | ✅ `wow.rs` (Walk-up + Windows-Registry), `exe.rs` (Version/Build), `relocate.rs` (Manager in WoW-Ordner verschieben) |
| **MVP-0 — vollständig** | ✅ Tree-Hash (`crates/tree-hash`), `.toc`-Parser (`crates/toc`), Scanner + Hash-Cache (`addons.rs`), Addon-Liste (`AddonTable.vue`) |
| MVP-1 und später | ⬜ |

---

## Motivation

Die Vanilla-Community spielt auf 1.12.1-Private-Servern. Der Addon-Bestand ist
heterogen:

- Viele Addons sind **nicht auf GitHub**, sondern liegen nur auf den Festplatten
  einzelner Spieler.
- Die existierende Welt liefert **viele Varianten desselben Addons** (Forks,
  Patches, persönliche Hacks), oft mit identischer `## Version:`-Angabe in der
  `.toc`-Datei aber unterschiedlichem Code.
- Manche Versionen enthalten Bugs, manche Bugfixes — ohne zentrale Stelle, die
  das nachverfolgt.
- Bestehende Manager (Instawow, WowUp, Ajour) sind auf 1.12.1 schwach kalibriert
  und kennen keinen P2P-Discovery, keine Curator-Workflows, keinen sauberen
  Push-Pfad zurück nach GitHub.

Ziel: ein Werkzeug, das alle vorhandenen Addon-Varianten via P2P einsammelt,
durch menschliche Curation in versionierte Git-Repos überführt, und Spielern
empfiehlt, welche Variante sie installieren sollten.

## Ziele

1. **Cross-Platform Desktop-App** für Windows, Linux, macOS (eine
   `.exe`/`.AppImage`/`.app`).
2. **Portable**: lauffähig aus beliebigem Pfad, idealerweise im WoW-Ordner.
3. **Install / Update / Rollback** von Addons aus Git-Quellen mit
   Inhalts-Verifikation.
4. **P2P-Discovery** vorhandener Addon-Varianten zwischen Spielern.
5. **Zentralisierter Collector-Service**, der entdeckte Varianten in Git-Repos
   archiviert.
6. **Curator-Workflow** für menschliche Review (Whitespace-Duplikate, Schadware,
   neue Versionen).
7. **Update-Tree** pro Addon, der Varianten klassifiziert (recommended / safe /
   broken / etc.) und Upgrade-Pfade definiert.
8. **Consumer- und Developer-Modus** pro Addon: ZIP-Install vs Git-Checkout.
9. **Selbst-Update** des Managers via signierter Releases.
10. Offen für **Multi-Curator-Welt** — andere Communities können eigene
    Indizes/Collectors betreiben, der Manager unterstützt mehrere Subscriptions.

## Nicht-Ziele

- Kein Retail/Classic-Cata/Wrath-Support. Scope bleibt 1.12.1.
- Kein Katalog-basierter Store à la CurseForge. Quelle ist immer eine Git-URL.
- Keine Mobile-/Browser-App.
- Keine Memory-/Injection-Eingriffe ins Spiel.
- Keine Identitäts-/Authentifizierungs-Schicht beim Client (vorerst). Spätere
  zentrale Login-Funktion ist eigene Phase.

---

## Vokabular

| Begriff | Bedeutung |
| --- | --- |
| **Variante** | Eine konkrete Snapshot-Version eines Addons, identifiziert durch ihren kanonischen Tree-Hash. |
| **Tree-Hash** (`tree_sha`) | `toa-tree-v1`: SHA-256 über den *normalisierten* Datei-Baum. Identitäts-Anker einer Variante. |
| **Git-Tree-SHA** (`git_tree_sha`) | Nativer SHA-1-Tree-Hash einer Git-Quelle. Ausschließlich Change-Detection-Cache, **nie** Identität. |
| **Addon-ID** | Stabiler Slug pro Addon, gleich dem Folder-Namen unter `Interface/AddOns/`. |
| **Index** | Kuratierte Liste aller bekannten Addons + ihrer Varianten + Empfehlungen. Lebt als Git-Repo. |
| **Update-Tree** | Pro Addon: Liste der Varianten + Status + Upgrade-Transitions. Teil des Index. |
| **Collector** | Service, der Manifeste empfängt, Content abholt, in Staging hält. |
| **Curator** | Mensch, der über die Curator-UI Varianten reviewt und in den Index promoted. |
| **Manifest** | Metadaten zu einer Variante (Tree-Hash, Toc-Daten, Provenienz). Kein Code. |
| **Consumer-Mode** | Addon installiert als entpacktes ZIP, kein lokaler Git-State. |
| **Developer-Mode** | Addon installiert via `git clone`, Working-Tree bearbeitbar/pushbar. |

---

## Drei-Rollen-Modell

```
┌────────────┐   manifest +    ┌────────────┐   curator    ┌────────────┐
│  Client    │   known_remotes │ Collector  │   review     │  Curator   │
│  (Spieler) │ ──────────────► │ (Service)  │ ◄──────────► │  (Mensch + │
│            │   P2P broadcast │            │              │   Web-UI)  │
└─────┬──────┘                 └──────┬─────┘              └─────┬──────┘
      │                               │                          │
      │ subscribe                     │ accept                   │ publish
      ▼                               ▼                          ▼
┌─────────────────────────────────────────────────────────────────────┐
│                          Git-Repos                                   │
│  ┌──────────────┐  ┌────────────────┐  ┌────────────────┐           │
│  │ addon-index  │  │ addon-pfUI     │  │ addon-Bagnon   │  ...      │
│  │ (Empfehlungen│  │ (alle Variants │  │                │           │
│  │  pro Addon)  │  │  dieses Addons)│  │                │           │
│  └──────────────┘  └────────────────┘  └────────────────┘           │
└─────────────────────────────────────────────────────────────────────┘
```

### Client (Spieler-App)

- Tauri-Desktop-App, lauffähig aus WoW-Ordner.
- Scannt `Interface/AddOns/`, berechnet den kanonischen Tree-Hash pro Addon (mit
  Hash-Cache).
- Ordnet Tree-Hashes gegen abonnierte Indizes zu, zeigt Status (recommended /
  safe / broken / unbekannt).
- Broadcastet eigene Manifeste + bekannte Git-Remotes ins P2P-Netz.
- Liefert Content (ZIP-Bytes) on-demand an Peers / Collector.
- Installiert ausschließlich Varianten aus vertrauten Indizes (Default:
  `vanilla-wow-dev/addon-index`). Im Advanced-Modus auch direkte Git-URLs oder
  rohe P2P-Funde.
- Bietet Consumer-Mode (ZIP-Install) und Developer-Mode (Git-Checkout) pro Addon.
- Self-Update via Tauri-Updater + Ed25519-Signaturen.

### Collector (Service, „lauschender Roboter")

- Headless Rust-Daemon (axum + iroh-Node + Git-Tooling).
- **Drei Discovery-Quellen** (parallel):
  1. **P2P-Broadcasts** von Clients (Manifeste mit Tree-Hash + bekannten Remotes).
  2. **Git-Poll** auf alle bekannten Remote-URLs (periodisch).
  3. **Auto-Track**-Konfiguration (Curator pflegt Liste fixer Repos).
- Holt unbekannten Content vom broadcastenden Peer oder via Git-Clone.
- Verifiziert den Tree-Hash durch Neuberechnung.
- Legt jede neue Variante in **Staging-DB** ab (kein automatischer Git-Commit).
- Erzeugt automatisch ein neues Addon-Repo (`vanilla-wow-dev/addon-<name>`), wenn
  `addon_id` noch unbekannt ist.
- Stellt Web-API für die Curator-UI bereit.

### Curator (Mensch + Web-UI)

- Web-Frontend auf dem Collector-Server.
- Zeigt Staging-Einträge mit Diff-Vorberechnung gegen bekannte Varianten.
- Aktionen pro Eintrag:
  - `Mark as duplicate of <variant>` (Whitespace-only erkannt).
  - `Blacklist` (Schadware).
  - `Accept as new variant` (commit als Orphan-Branch im Addon-Repo,
    Index-Eintrag).
  - `Ignore` (uninteressant, aber kein Spam).
- Bulk-Operations für Variant-Cluster.
- Tree-Editor: Status pro Variante setzen (recommended / safe / experimental /
  deprecated / broken / malicious / unreviewed). Transitions definieren
  („von X upgrade zu Y wegen Bugfix").
- `master`-Branch des Addon-Repos auf eine Variante als „recommended" promoten.

---

## Drei Repo-Typen

### 1. Index-Repo (`vanilla-wow-dev/addon-index`)

Trust-Anchor. Nur dieser Index wird vom Manager standardmäßig vertraut.

```
addon-index/
├── README.md
├── catalog.json                      # Optional: aggregierte Liste aller addon_ids
└── addons/
    ├── pfUI.json                     # Eintrag pro Addon
    ├── Bagnon.json
    └── ...
```

Pro Addon eine Datei → kleine PR-Diffs, gut kuratierbar.

### 2. Addon-Repo (`vanilla-wow-dev/addon-<id>`)

Ein Repo pro Addon, enthält alle bekannten Varianten als Orphan-Branches.

```
vanilla-wow-dev/addon-pfUI/
├── refs/heads/master                  # = aktuelle recommended Variante (Files an root)
├── refs/heads/variant/abc123          # Orphan: Files dieser Variante
├── refs/heads/variant/def456          # Orphan: andere Variante
├── refs/tags/recommended              # movable, zeigt auf master
├── refs/tags/variant-abc123           # immutable, zeigt auf variant/abc123
└── refs/tags/v3.1.0-shagu             # semantisches Alias
```

Branch- und Tag-Namen verwenden die ersten 12 Hex-Zeichen des kanonischen
Tree-Hash.

GitHub-Releases enthalten ZIPs:
- `pfUI-recommended.zip` (von `master`, rolling).
- `pfUI-variant-abc123.zip` (immutable, pro Variante).

Releases werden via GitHub-Action automatisch beim Branch-Push erzeugt.

**Repo-Hygiene (verbindlich für alle vom Collector erzeugten Repos):** Jedes
Addon-Repo bekommt eine `.gitattributes` mit `* -text` und alle Blobs werden mit
Mode `100644` committet. Damit sind Working-Tree-Bytes und Object-Store-Bytes
immer identisch, unabhängig von der `core.autocrlf`-Einstellung des Klonenden.

### 3. Collector-Staging (intern, nicht öffentlich)

Datenbank + Filesystem auf dem Collector-Host. Persistiert ungereviewte Varianten
bis zur Curator-Entscheidung.

```
collector-data/
├── staging.db                         # SQLite mit Variant-Metadaten
├── staging-content/
│   └── <staging-uuid>/                # Files pro Staging-Eintrag
└── tracking.json                      # bekannte Remote-URLs pro addon_id
```

---

## Drei Quell-Typen für Installs

Lockfile-frei. Jede installierte Variante hat eine implizite Quelle, die aus dem
Working-Tree (oder aus Index-Lookup) ableitbar ist.

```jsonc
// "indexed" — kuratierte Variante aus einem vertrauten Index
{ "kind": "indexed",
  "index_url": "https://github.com/vanilla-wow-dev/addon-index",
  "addon_id": "pfUI",
  "tree_sha": "abc123..." }

// "direct-git" — beliebige Git-URL, vom User selbst hinzugefügt
{ "kind": "direct-git",
  "url": "git@github.com:shagu/pfUI.git",
  "ref": "refs/heads/master",
  "commit": "fed987...",
  "tree_sha": "abc123..." }

// "p2p" — Tree-Hash ist Identität, Quelle nur Transport-Hinweis
{ "kind": "p2p",
  "tree_sha": "abc123...",
  "fallback_url": "https://github.com/vanilla-wow-dev/addon-pfUI",
  "fallback_ref": "refs/heads/variant/abc123" }
```

---

## Identität & Versions-Modell

### Content-First-Identity

```
canonical_release = (addon_id, tree_sha)
```

- **`addon_id`** = Folder-Name unter `Interface/AddOns/`. Stabile Convention.
- **`tree_sha`** = kanonischer Tree-Hash `toa-tree-v1` (siehe unten).

Der `## Version:`-String aus der `.toc`-Datei ist **reine Display-Metadata**,
niemals Identitäts-Kriterium.

### Der kanonische Tree-Hash: `toa-tree-v1`

**Warum nicht einfach `git hash-object -t tree`?** Weil Git nicht die
Working-Tree-Bytes hasht, sondern den *normalisierten* Inhalt im Object-Store.
`core.autocrlf` und `.gitattributes text=auto` sind Checkout-Filter: beim
Einchecken wird auf LF normalisiert, beim Auschecken plattformspezifisch
expandiert. Git ist stabil, *weil* es normalisiert — nicht wegen des Hashes.

Tome of Addons hasht aber Working-Trees direkt, einschließlich entpackter ZIPs
ganz ohne Git-Kontext. Also muss die Normalisierung hier stattfinden. Ohne sie
hätte dasselbe Addon je nach Betriebssystem des Spielers eine andere Identität —
und damit wäre der gesamte Index wertlos.

#### Normalisierungsregeln

| Achse | Divergenz zwischen Betriebssystemen | Regel |
| --- | --- | --- |
| Zeilenenden | Windows-Checkout expandiert LF → CRLF | Text-Dateien: **CRLF → LF**, dann hashen. Ein **einzelnes CR bleibt unangetastet** — kein OS-Filter erzeugt je eines, es ist also echter Inhalt; es umzuschreiben würde Dateien beschädigen, die legitim ein CR enthalten. Binär-Erkennung wie Git: NUL-Byte in den ersten 8000 Bytes ⇒ binär ⇒ Bytes roh |
| Exec-Bit | Windows kennt keins (`100644` vs `100755`) | Mode **immer `100644`** |
| Symlinks / Submodules | Semantik plattformabhängig, unter Wine unzuverlässig | Werden **nicht** gehasht — ihr Vorhandensein ist ein Fehler, kein still ignorierter Sonderfall |
| Dateiname-Unicode | macOS liefert NFD, Linux/Windows NFC | Namen vor dem Hash auf **NFC** normalisieren |
| Case-Kollision | `Foo.lua` + `foo.lua` existieren nur auf case-sensitiven FS nebeneinander | **Fehler melden**, nicht still falsch hashen |
| OS-Artefakte | `.DS_Store`, `Thumbs.db`, `desktop.ini`, `._*` | Ausschließen |
| Git-Metadaten | `.git/` im Developer-Mode vorhanden, im Consumer-Mode nicht | `.git/` komplett ausschließen |
| Pfad-Trenner | `\` vs `/` | Immer `/` |
| Leere Verzeichnisse | Git kann sie nicht darstellen | Ignorieren |

#### Serialisierung

Einträge werden im Git-Tree-Objektformat serialisiert und nach Git-Regel sortiert
(byteweise über den Namen, Verzeichnisse mit implizitem `/`). Über diese
Serialisierung läuft **SHA-256**. Anzeige: erste 12 Hex-Zeichen.

Die Git-kompatible Struktur bleibt erhalten, weil sie nachprüfbar korrekt und gut
verstanden ist — nur Hash-Funktion und Vor-Normalisierung weichen ab.

#### Warum SHA-256 statt SHA-1

Git-Kompatibilität ist durch die Normalisierung ohnehin aufgegeben — bei einem
Repo, das CRLF im Object-Store hält (bei alten WoW-Addon-Repos nicht selten),
weicht der Hash ab. Damit entfällt der einzige Grund für SHA-1.

Und SHA-1 ist hier nicht folgenlos: der Tree-Hash ist laut Trust-Modell der
*Sicherheits*-Anker („MITM kann den Hash nicht ändern, ohne den Content zu
kennen"). Chosen-Prefix-Kollisionen auf SHA-1 sind seit 2020 praktisch
bezahlbar — ein Angreifer könnte eine Schadware-Variante konstruieren, die auf
einen kuratierten Hash kollidiert. Für einen neu entworfenen Identitäts-Anker
wäre das ein vermeidbarer Konstruktionsfehler.

#### Dual-Hash

Der native Git-Tree-SHA behält genau eine Aufgabe: Der Poll-Daemon kann ihn über
die GitHub-API lesen, **ohne zu klonen**, und damit billig erkennen, ob sich
etwas geändert hat.

| Feld | Algorithmus | Rolle |
| --- | --- | --- |
| `tree_sha` | `toa-tree-v1` (SHA-256, normalisiert) | **Identität** — steht im Index, in Branch-Namen, in Manifesten |
| `git_tree_sha` | nativ (SHA-1), nur bei Git-Quellen | **Change-Detection / Poll-Cache** — nie Identität, nie Trust-Grundlage |

#### Performance

Gemessen an einem realen Bestand (259 Addon-Ordner, 10.364 Dateien, 434,6 MB,
davon 192,5 MB in CRLF-Dateien) auf einem Intel i7-7820HQ (Kaby Lake, 2017) —
**ohne** SHA-NI-Hardwarebeschleunigung, also der realistische Worst Case für die
Zielgruppe. Warme Page-Cache, Release-Build, ein Thread:

| Phase | Zeit |
| --- | --- |
| Reiner Verzeichnis-Walk (nur `stat`) | 0,041 s |
| Alle Dateien lesen | 0,253 s |
| Lesen + CRLF-Scan | 0,339 s |
| Lesen + SHA-256 (`sha2`-Crate) | 3,427 s |
| **Vollständiger `hash_tree` über alle 259 Ordner** | **3,62 s** |

Die Aufschlüsselung ist eindeutig: Walk, Normalisierung und Serialisierung
kosten zusammen rund 0,2 s. Alles andere ist die Hash-Funktion selbst. Die
`sha2`-Crate erreicht hier 137 MB/s, OpenSSLs `sha256sum` 300 MB/s — Faktor 2,2,
weil auf diesem Rechner der Software-Fallback greift. Auf CPUs mit SHA-NI
(AMD Zen durchgängig, Intel ab Ice Lake) schaltet `sha2` über `cpufeatures`
automatisch auf die Hardware-Instruktionen um.

Das `asm`-Feature von `sha2` wurde gemessen und verworfen: auf x86_64 praktisch
wirkungslos (3,72 s statt 3,62 s), würde aber eine C-Toolchain in jeden
Cross-Build zwingen.

Einordnung: 3,6 s einmalig auf dem schlechtestmöglichen Rechner. Der
tatsächlich gemessene End-to-End-Scan über `addons.rs` liegt dank `rayon` bei
**1,70 s kalt** und bei **0,010 s warm** — Faktor 175 durch den Hash-Cache.

Implementierungsauflagen:
- Parallelisierung über `rayon` — gehört in den Addon-Walk (`addons.rs`), nicht
  in den Hasher; die Crate bleibt bewusst single-threaded und damit trivial
  testbar.
- CRLF→LF-Normalisierung **run-weise**, nicht byteweise: eine Push-pro-Byte-
  Schleife kostete auf dem Korpus messbar 3,4 s zusätzlich (6,99 s statt 3,62 s).
- Dateien über 16 MB werden **streamend in zwei Durchläufen** gehasht statt in
  den Speicher gelesen — im Bestand liegen Einzeldateien von 80 MB. Der erste
  Durchlauf ermittelt die normalisierte Länge für den Objekt-Header, der zweite
  füttert den Digest aus dem gerade gewärmten Page-Cache. Ein CR an der
  Puffergrenze wird zurückgehalten, bis der nächste Chunk zeigt, ob ein CRLF
  beginnt.
- Binär-Erkennung liest die ersten 8000 Bytes, entscheidet, streamt dann weiter.

### Versions-Problem

Beobachtbare Konstellationen pro Addon werden vom Manager so klassifiziert:

### Client-Bezug: Interface-Version

WoW lädt ein Addon nur, wenn dessen `## Interface` zur Interface-Version des
Clients passt — sonst erst nach Aktivieren von „Veraltete AddOns laden". Der
Manager gleicht deshalb beides ab und markiert Abweichungen.

Im vermessenen Bestand betrifft das **11 von 242 Addons** (4× `11100`,
7× `11000` gegen `11200`). Ohne diese Anzeige sucht der Nutzer den Fehler beim
Addon statt bei der Interface-Version.

| Lokal vs Index | Reaktion |
| --- | --- |
| Selber Tree-Hash bekannt im Index, Status `recommended` | OK, kein Update nötig. |
| Anderer Tree-Hash bekannt im Index, Transition `from→to` definiert | „Upgrade auf <Y> empfohlen, Grund: …". |
| Tree-Hash bekannt, Status `broken/malicious` | UI markiert rot, fordert zum Update auf. |
| Tree-Hash unbekannt im Index | „Unbekannte Variante — broadcast an Collector?" |
| Lokaler Tree-Hash != letzter berechneter Hash bei selbem Folder | „Lokale Änderung erkannt." (Developer-Mode-Indikator) |

Display-Versions-Strings werden nur informativ neben dem Hash gezeigt:
`pfUI 3.1.0 · abc123…`.

---

## Schemas

### Manifest (Client → P2P → Collector)

```jsonc
{
  "schema_version": 1,
  "addon_id": "pfUI",
  "hash_algo": "toa-tree-v1",
  "tree_sha": "abc123def456...",
  "git_tree_sha": "fed987...",        // optional, nur bei Git-Quelle
  "size_bytes": 384572,
  "toc": {
    "title": "pfUI",
    "version": "3.1.0",
    "interface": "11200",
    "notes": "..."
  },
  "known_remotes": [
    { "url": "git@github.com:shagu/pfUI.git",
      "ssh_alt": "https://github.com/shagu/pfUI",
      "branch": "master",
      "commit": "fed987..." }
  ],
  "first_seen_locally": "2026-04-20T...",
  "manager_version": "0.2.1"
}
```

Keine Signatur. Identität ist `tree_sha`, Verifikation per Recompute nach
Content-Pull.

`hash_algo` ist Pflichtfeld: Ein künftiger Algorithmus-Wechsel darf nicht dazu
führen, dass alte und neue Hashes stillschweigend verglichen werden.

### Index-Eintrag pro Addon

```jsonc
// addon-index/addons/pfUI.json
{
  "schema_version": 1,
  "addon_id": "pfUI",
  "canonical_repo": "https://github.com/vanilla-wow-dev/addon-pfUI",
  "hash_algo": "toa-tree-v1",
  "install": {
    "kind": "release-zip",
    "asset_pattern": "pfUI-variant-*.zip"
  },
  "current_recommended": "abc123...",
  "variants": [
    {
      "tree_sha": "abc123...",
      "git_ref": "refs/heads/variant/abc123",
      "git_commit": "fed987...",
      "declared_version": "3.1.0",
      "label": "v3.1.0 (Shagu official)",
      "status": "recommended",
      "first_seen": "2026-04-10T...",
      "size_bytes": 384572
    },
    {
      "tree_sha": "789xyz...",
      "git_ref": "refs/heads/variant/789xyz",
      "declared_version": "3.1.0",
      "label": "v3.1.0 broken",
      "status": "broken",
      "reason": "crash on /reload, siehe issue #42"
    }
  ],
  "transitions": [
    { "from": "789xyz...", "to": "abc123...",
      "kind": "fix", "priority": "high",
      "reason": "Bugfix für Crash-on-Reload" }
  ],
  "blacklisted_sources": []
}
```

Status-Werte: `recommended`, `safe`, `experimental`, `deprecated`, `broken`,
`malicious`, `unreviewed`.

### Install-Instruction-Kinds

| `kind` | Bedeutung |
| --- | --- |
| `release-zip` | ZIP aus GitHub-Releases, `asset_pattern` matcht Datei. |
| `git-root` | Repo-Root selbst ist der Addon-Folder. |
| `git-subfolder` | Ein Subfolder im Repo ist der Addon-Folder. |
| `git-subfolders` | Mehrere Subfolders → mehrere AddOns gleichzeitig. |
| `external-manifest` | Author pflegt `.addon-install.json` im Repo, Manager folgt der. |

Externes Manifest (vom Author kontrolliert):

```jsonc
// .addon-install.json im Repo-Root
{
  "schema_version": 1,
  "folders": [
    { "source": "src/main",   "target": "ShaguTweaks" },
    { "source": "src/extras", "target": "ShaguTweaks_Extras" }
  ]
}
```

### Staging-Eintrag (Collector-intern)

```jsonc
{
  "id": "uuid",
  "addon_id": "pfUI",
  "tree_sha": "789xyz...",
  "received_at": "2026-04-25T...",
  "size_bytes": 384572,
  "providers": [
    { "kind": "p2p-broadcast", "node_id": "iroh:abc...", "remote_hint": "git@github.com:fork/pfUI.git" },
    { "kind": "git-poll", "url": "https://github.com/fork/pfUI", "branch": "master", "commit": "..." }
  ],
  "toc_metadata": { ... },
  "diff_against_known": [
    { "ref_hash": "abc123...",
      "lines_added": 0, "lines_removed": 0, "lines_modified": 3,
      "structural_similarity": 0.9998,
      "guess": "whitespace-only" }
  ],
  "review_status": "pending",
  "review_decision": null,
  "reviewer": null,
  "reviewed_at": null
}
```

Anmerkung: Reine Zeilenenden-Unterschiede erzeugen durch die Normalisierung gar
keine neue Variante mehr — sie kommen im Staging nicht an. Die
Whitespace-Duplikat-Erkennung bleibt trotzdem nötig für Einrückungs- und
Trailing-Whitespace-Varianten.

---

## Discovery-Mechanik (Collector-Seite)

Drei Quellen, gemeinsamer Staging-Eingang:

### 1. P2P-Broadcast (iroh-gossip)

- Topic: `wow-addons-1.12.1`.
- Peers haben ephemere iroh-Node-IDs (kein User-Identity).
- Beim Start: Manager broadcastet alle eigenen Manifeste.
- Bei Addon-Änderung: Re-Broadcast.
- Bootstrap-Ticket im Manager-Binary hardcoded, in Settings überschreibbar.

### 2. Git-Poll-Daemon

- Periodisch `git ls-remote` auf alle bekannten `(addon_id, remote_url)`-Paare.
- Default 1 h Intervall, adaptiv (auf 24 h erhöhen, wenn lange keine Änderung).
- Bei neuem Commit → shallow clone → Tree-Hash → Staging.
- `git_tree_sha` dient als Cache-Key: unverändert ⇒ kein Clone nötig.
- Tracking-DB persistent.
- Dead-Detection: 7 Tage 404 → pausieren.

### 3. Auto-Track-Konfiguration

```jsonc
{
  "tracked_repos": [
    {
      "addon_id": "pfUI",
      "url": "https://github.com/shagu/pfUI",
      "branch": "master",
      "poll_interval_sec": 3600,
      "default_label": "shagu-master-{commit_short}",
      "default_status": "experimental"
    }
  ]
}
```

Curator pflegt manuell für unverzichtbare Repos, die nie via P2P entdeckt würden.

### Auto-Repo-Creation

Bei unbekanntem `addon_id` im Staging:
1. Collector ruft `POST /orgs/vanilla-wow-dev/repos` (GitHub-App-Token).
2. Repo `vanilla-wow-dev/addon-<sanitized-id>` mit master + Template-README +
   `.gitattributes` (`* -text`).
3. Erste Variante als `variant/<short>` Orphan-Branch.
4. Index-Eintrag `addons/<id>.json` mit Status `unreviewed`.
5. Curator kann später umbenennen / löschen / mergen.

---

## Client-Architektur

### Tauri-Stack

- **Backend**: Rust mit `tokio`, `sha2` + `rayon` für den Tree-Hash, System-`git`
  für Clone/Fetch (SSH-Auth über die OS-Konfiguration), `iroh` für P2P,
  `octocrab` für GitHub-API, `serde` für Schemas.
- **Frontend**: Vue 3 + Vite + TypeScript + Tailwind + TanStack Table (headless).
- **IPC**: `#[tauri::command]` für Sync/Async-Calls, EventBus für Progress-Streams.

### Lokale Persistenz (kein Lockfile)

Was gespeichert wird:

```
<app-config-dir>/dev.vanilla-wow.tome-of-addons/   (oder ./data/ im --portable)
├── settings.json                       # WoW-Pfad, Indizes, P2P-Konfig, OAuth-Tokens, UI-Prefs
├── cache/
│   ├── tree-hashes/<wow-root-id>.json  # Hash-Cache pro Addon-Folder
│   ├── indices/<index-name>/           # Pull der index.json + addons/*.json
│   ├── github-trees/<repo-id>.json     # ETag-basierter Cache von ls-remote-Ergebnissen
│   └── peers/                          # iroh-Bootstrap, Last-seen-Peers
└── logs/
```

**Was NICHT gespeichert wird:** Liste installierter Addons, Versionen, „was wurde
wann installiert". Die Wahrheit ist immer die Festplatte: scan + hash +
Index-Lookup.

### Hash-Cache-Invalidierung

Pro Addon-Folder wird gespeichert: `(file_count, max_mtime, total_bytes)`. Bei
Scan:

- Wenn das Tripel unverändert ist → Cache-Hit, alten Tree-Hash verwenden.
- Sonst rehashen.

`total_bytes` ist gegenüber dem ursprünglichen `(file_count, max_mtime)` ergänzt:
Der Verzeichnis-Walk liest die Größe ohnehin mit (gemessen 41 ms für 10.364
Dateien), und das Tripel fängt zusätzlich den Fall „Datei geändert, mtime
erhalten" ab.

Skaliert auf 1000+ Addons in Sekunden statt Minuten.

### Mode-Detection

`.git/`-Verzeichnis im Addon-Folder vorhanden → Developer-Mode, sonst
Consumer-Mode.

In Developer-Mode liest Manager `.git/config`:
- `remote.origin.url` → Quelle.
- Aktueller Branch + Commit → Display.

### Install-Flow

```
User klickt "Install" auf Addon X (aus Index)
  │
  ├── Dialog: Mode-Wahl
  │     - Consumer (Default): ZIP-Download
  │     - Developer: Git-Clone (URL überschreibbar für eigene Forks)
  │
  ├── Mode = Consumer:
  │     1. ZIP von GitHub-Release laden
  │     2. SHA-256 verifizieren (aus Release-Notes oder Index)
  │     3. Entpacken nach Interface/AddOns/<addon_id>/
  │     4. Tree-Hash neu berechnen, gegen erwarteten matchen
  │
  └── Mode = Developer:
        1. git clone <url> Interface/AddOns/<addon_id>
        2. Auf gewünschten Variant-Branch checkouten
        3. Tree-Hash berechnen, verifizieren
```

Jeder vom Manager ausgeführte Clone setzt `core.autocrlf=false` und
`core.eol=lf`. Das ist zwar durch die Hash-Normalisierung nicht mehr
sicherheitskritisch, verhindert aber, dass WoW selbst je nach Plattform
unterschiedliche Bytes zu sehen bekommt.

P2P-Fallback bei beiden Modi: wenn GitHub langsam/unavailable, iroh-Blob vom Peer
ziehen.

### Update-Flow

```
Periodisch (z.B. stündlich) + on-demand:
  1. Index pullen (cached mit ETag).
  2. Für jedes installierte Addon:
       a. Tree-Hash aus Cache holen.
       b. Im Index: Status prüfen.
       c. Transitions checken: gibt es ein „from = lokaler_hash"-Transition?
       d. Falls ja: UI-Notification "Upgrade verfügbar".
  3. User klickt "Update" → Install-Flow für Ziel-Hash.
```

In Developer-Mode kein automatisches Update. Manager zeigt nur „Remote hat neuen
Commit, willst du `git pull` machen?".

### Developer-Panel

```
┌─ pfUI (Developer-Mode) ──────────────────────────────┐
│ Repo:    git@github.com:shagu/pfUI.git               │
│ Branch:  master                                      │
│ Status:  ✓ clean                                     │
│ Remote:  ↑ 2 ahead, ↓ 0 behind                       │
│                                                      │
│ Tree-Hash (working): abc123…                         │
│ Im Index:            bekannt als "shagu-master"      │
│ Status:              experimental                    │
│                                                      │
│ [Pull] [Push] [Status] [Open Folder] [Open Terminal] │
│                                                      │
│ Branches: [master ▼]                                 │
└──────────────────────────────────────────────────────┘
```

Manager macht nur die produktivitäts-relevanten 80-%-Operationen.
Diff-Viewer/Rebase/Merge → Editor/Terminal.

### Git-Backend-Wahl

Der Tree-Hash braucht **keine** Git-Bibliothek — er wird direkt aus dem
Dateisystem berechnet (`sha2` + eigener Serializer). Damit fällt die ursprüngliche
Abwägung `git2` vs `gix` für den Identitäts-Pfad komplett weg.

Für Clone/Fetch/Push gilt:
- **System-Git als Default**, wenn vorhanden (probe `git --version` beim Start).
  Nutzt `~/.ssh/config`, `ssh-agent`, OS-Keyring → kein Manager-spezifisches
  Setup nötig.
- Eine Rust-Git-Bibliothek wird erst evaluiert, wenn ein konkreter Fall System-Git
  ausschließt (z. B. Windows ohne Git-Installation). Offener Punkt, siehe unten.

### Self-Update

- `tauri-plugin-updater` checkt bei Start + alle 24 h.
- Ed25519-Signaturen für Binaries, Public-Key im Binary einkompiliert.
- Update wird heruntergeladen + beim nächsten App-Start appliziert. Persistente
  UI-Banner-Notification: „Update bereit — bitte App neu starten". Kein
  Silent-Apply.
- Alte Binary als `.old` für Rollback.

CI/CD siehe [`../README.md`](../README.md) — release-please + `tauri-action`.

---

## Collector-Architektur

### Stack

- Headless Rust-Daemon, deployt als Container oder Systemd-Service.
- **Web-Server**: `axum` für REST-API + statische Curator-UI.
- **P2P-Listener**: iroh-Node, abonniert auf Topic.
- **Git-Tooling**: System-Git für Clone/Push, eigener Hasher für die Identität.
- **GitHub-API**: GitHub-App mit Scopes `repos:write`, `metadata:read`.
- **Storage**: SQLite für Staging-DB + Tracking, FS für Content-Blobs.

### Workflow pro neuem Manifest

```
1. P2P-Manifest empfangen ODER Git-Poll erkennt neuen Commit.
2. Tree-Hash-Lookup: schon im Index oder in Staging?
   - Ja → Provenienz erweitern (welche Source kennt diese Variante).
   - Nein → weiter.
3. Content holen:
   - P2P-Pfad: iroh-Blob-Anfrage an broadcastenden Peer.
   - Git-Pfad: shallow clone des Commits.
4. Tree-Hash recompute, gegen erwarteten verifizieren.
5. Bei Mismatch: Source markieren als unzuverlässig, ignorieren.
6. Bei Match: Staging-Eintrag mit Diff-Vorberechnung anlegen.
7. addon_id unbekannt → Auto-Repo-Creation triggern.
```

### Anti-Spam

- Rate-Limit pro iroh-Node-ID (z. B. 100 Manifeste/h).
- Größenlimit pro Manifest (1 KB) und pro Content-Pull (10 MB).
- Stale-Detection: identische Provenienz-Source mehrfach binnen 1 h → throttle.
- Fake-Remote-Schutz: gepulltes Repo passt nicht zu `addon_id` (z. B. komplett
  andere `.toc`-`title`) → Source-Blacklist für diese `addon_id`.

---

## Curator-Architektur

### Web-UI (auf Collector gehostet)

- Auth: GitHub-OAuth, Permissions an `vanilla-wow-dev`-Org-Membership gekoppelt.
- Diff-Viewer: side-by-side Lua mit Syntax-Highlighting.
- Variant-Cluster-View: Whitespace-only-Cluster automatisch gruppiert.
- Bulk-Actions („mark these 12 as duplicates of X").
- Tree-Editor: Status-Vergabe pro Variante, Transitions definieren.
- Auto-Track-Konfig-Editor.

### Action-Effekte

| Aktion | Effekt |
| --- | --- |
| Mark as Duplicate | Staging-Eintrag verknüpft mit Original-Variante, kein Git-Commit. |
| Blacklist | Hash kommt in `index/blacklist.json`, Staging-Eintrag closed. |
| Accept | Files committed als `variant/<short>` Orphan-Branch im Addon-Repo, Index-Eintrag aktualisiert (Status default `safe` oder `experimental`). |
| Promote to Recommended | `master` des Addon-Repos auf Variant-Tree force-pushed, GitHub-Action erzeugt neues Release-ZIP, Index `current_recommended` aktualisiert. |
| Ignore | Staging-Eintrag closed ohne weitere Aktion. |

---

## Schutz / Trust-Modell

### Default-Verhalten

| Vektor | Verteidigung |
| --- | --- |
| Bösartiger Peer broadcastet Schadcode-Variante | Default-Index führt sie nicht. UI zeigt sie nicht. Kein Install. |
| Bösartiger Peer übersättigt Collector mit Junk | Rate-Limit + Größenlimit. Curator kann Source blacklisten. |
| Bekannte Schadware-Hash | Index `status: malicious` → Manager warnt aktiv, falls lokal installiert. |
| Bösartiger Index | Default nur `vanilla-wow-dev/addon-index` trusted. Andere Indizes erfordern explizite User-Aktion. |
| Curator-Compromise | Index-Repo ist Git → Audit-Log da. Reverse-Commit möglich. Optional: Commit-Signing am Index. |
| Variant-Explosion (Whitespace) | Zeilenenden erzeugen durch Normalisierung gar keine Varianten mehr; für den Rest gruppiert Auto-Diff im Staging + Bulk-„mark as duplicate". |
| MITM auf Manifest | Manifest enthält den Tree-Hash, Content wird beim Pull nachgerechnet. SHA-256 ⇒ keine praktikable Kollision. |
| Manager-Update kompromittiert | Ed25519-Signatur. Public-Key im Binary kompiliert. Kein Self-Update aus unsignierter Quelle. |

### Trust-Hierarchie im Manager

1. **Default-Trust**: nur `vanilla-wow-dev/addon-index` + der zugehörige Collector
   (im Build hardcoded).
2. **User-konfigurierbar**: weitere Indizes via Settings hinzufügbar (URL eingeben
   + expliziter „Trust"-Klick).
3. **P2P-Empfehlungen**: andere Peers können „Index-Vorschläge" gossipen, landen
   in „Suggested"-Liste, **niemals automatisch aktiv**.

### Install nur aus vertrauter Quelle

Im Default-Modus: nur Tree-Hashes installierbar, die in mindestens einem
vertrauten Index stehen. Hash nicht im Index → kein Install-Button.

Advanced-Modus (Opt-in): Discovery-Tab zeigt rohe P2P-Funde und beliebige
Git-URLs zur Direkt-Installation. Mit prominenter Warnung.

---

## Privacy & Opt-Outs

### Beim ersten Manager-Start

Disclosure-Dialog:
> „Der Manager broadcastet die Tree-Hashes deiner Addons + öffentliche
> Git-Remote-URLs an den Collector. Das hilft der Community, neue Varianten zu
> finden. Private Repos werden nicht geteilt. Alles deaktivierbar."

Buttons: „Erlauben" (default) | „Detail" | „Ablehnen".

### Granulare Opt-Outs in Settings

- Globaler Toggle: P2P-Broadcast an/aus.
- Pro Addon: „Diese Variante nicht broadcasten".
- Pro Remote: „Diese URL nicht teilen" + Whitelist/Blacklist von Hosts.
- „Inkognito-Modus": gar nichts broadcastet, nur subscribe.

### Public-vs-Private-Detection

Für SSH-URLs (`git@github.com:user/repo.git`):
- Manager probiert HTTPS-Variante anonym (`https://github.com/user/repo.git`).
- Erfolg → public, sicher zum Teilen.
- Fehler/Auth-Required → private, nicht teilen.

Hosts mit privatem Charakter (`git.internal.firma.de`, IP-Adressen, `*.local`)
per Default nicht teilen.

---

## Tech-Stack

### Client

| Layer | Tech |
| --- | --- |
| Shell / Window / Bundling | Tauri v2 |
| Backend-Sprache | Rust (stable) |
| Frontend-Sprache | TypeScript |
| Frontend-Framework | Vue 3 + Vite |
| Styling | Tailwind v4 (CSS-first, `@tailwindcss/vite`) |
| Tabellen | TanStack Table v8 (headless) |
| Schriften | Cinzel (Display) + EB Garamond (Fließtext), lokal eingebettet, SIL OFL 1.1 |
| Lokalisierung | vue-i18n (de / en / fr) |
| Async-Runtime | tokio |
| Hashing | `sha2` (SHA-256) + eigener Tree-Serializer |
| Parallelisierung | `rayon` |
| Unicode-Normalisierung | `unicode-normalization` (NFC) |
| Git (Clone/Fetch/Push) | System-`git` via subprocess |
| GitHub-API | octocrab |
| P2P | iroh (gossip + blobs) |
| Update-Plugin | tauri-plugin-updater (Ed25519-Signaturen) |
| Filesystem-Watcher | notify |
| Datenformat | serde + serde_json |

### Collector

| Layer | Tech |
| --- | --- |
| HTTP-Server | axum |
| P2P-Node | iroh |
| Git | System-git |
| Hashing | gleicher Tree-Hasher wie der Client (gemeinsame Crate) |
| GitHub-API | octocrab + GitHub-App |
| Storage | SQLite (sqlx) + lokales FS |
| Auth (Curator-UI) | GitHub-OAuth (oauth2-rs) |
| Frontend (Curator-UI) | Vue 3 + Vite + Tailwind (gleiche Konvention wie Client) |

Der Tree-Hasher muss zwingend als **gemeinsame Crate** von Client und Collector
genutzt werden. Zwei unabhängige Implementierungen desselben Normalisierungs-
Regelwerks würden früher oder später divergieren — und eine Divergenz im
Identitäts-Anker ist der teuerste denkbare Bug in diesem System.

### Repos

| Repo | Zweck |
| --- | --- |
| `vanilla-wow-dev/tome-of-addons` | Source-Code des Clients + Releases. |
| `vanilla-wow-dev/addon-collector` | Source-Code des Collector-Service + Curator-UI. |
| `vanilla-wow-dev/addon-index` | Kuratierter Index. Pull-Requests von Curators. |
| `vanilla-wow-dev/addon-<id>` | Pro Addon eines, vom Collector auto-created und gepflegt. |

---

## Phasen-Plan

### MVP-0 — Lokale Discovery ✅
- ✅ Tauri-Skeleton + Vue-Frontend.
- ✅ WoW-Installations-Erkennung (Walk-up + Registry) und Relocate.
- ✅ Scanner für `Interface/AddOns/`, Tree-Hash, Hash-Cache.
- ✅ `.toc`-Parser.
- ✅ Mode-Detection (`.git/` vs nicht).
- ✅ UI: Liste aller Addons mit Tree-Hash-Anzeige.

### MVP-1 — Direct-Git-Install
- Install via beliebiger Git-URL (HTTPS + SSH).
- System-Git-Backend mit SSH-Agent.
- Consumer-Mode (Clone + `.git` löschen) und Developer-Mode (Clone behalten).
- Update via `git fetch` + Tree-Hash-Vergleich.

### MVP-2 — Index-Subscription
- Hardcoded Index-URL (`vanilla-wow-dev/addon-index`).
- Index-Pull mit ETag-Cache.
- Lookup pro lokaler Variante: Status aus Index anzeigen.
- Update-Empfehlungen aus Transitions.

### MVP-3 — Self-Update ✅
- ✅ tauri-plugin-updater integriert.
- ✅ CI: GitHub-Actions Build + Release via release-please.
- ✅ Restart-Banner-UX.

### MVP-4 — Indexed Install + Mode-Wahl
- Variant-Branch-Fetch oder Release-ZIP-Download.
- Tree-Hash-Verifikation.
- Install-Dialog mit Consumer/Developer-Wahl + URL-Override für Forks.

### MVP-5 — Developer-Panel
- Status / Pull / Push / Open Folder / Open Terminal.
- Branch-Switch.
- Live-Update des Tree-Hash bei lokalen Änderungen.

### MVP-6 — P2P-Broadcast (Outbound)
- iroh-Node im Manager.
- Manifest-Broadcast inkl. `known_remotes`.
- Privacy-Disclosure-Dialog beim Erststart.

### MVP-7 — Collector
- Headless Service mit P2P-Listener.
- Staging-DB mit Diff-Vorberechnung.
- Curator-Web-UI (Login, Diff-View, Accept/Blacklist/Duplicate).
- Auto-Repo-Creation für unbekannte `addon_id`s.

### MVP-7.5 — Git-Poll-Daemon
- Tracking-DB für `(addon_id, remote_url)`.
- Periodisches `git ls-remote`, `git_tree_sha` als Cache-Key.
- Adaptive Polling.

### MVP-8 — P2P-Content-Fetch
- Iroh-Blob-Sharing.
- Manager nutzt P2P als Fallback, wenn GitHub unavailable.

### MVP-9 — Multi-Index + P2P-Index-Discovery
- Manager unterstützt mehrere abonnierte Indizes.
- Konflikt-Anzeige bei widersprüchlichen Empfehlungen.
- P2P-Gossip von Index-Empfehlungen, niemals auto-trusted.

### Phase 10+ — State-Sync und zentrale Login-Funktion
- Zentrales User-Login (Profile-Sync zwischen Rechnern).
- SavedVariables-Sync (Lua-aware Merge, Erweiterung des
  `Invite-o-matik/merge.lua`-Patterns).
- Auto-Commit von SavedVariables-Änderungen via Filesystem-Watcher.

---

## Entscheidungs-Log

| # | Entscheidung | Begründung |
| --- | --- | --- |
| E-1 | Identität ist `toa-tree-v1` (SHA-256 über normalisierten Baum), **nicht** `git hash-object -t tree` | Git hasht normalisierten Object-Store-Inhalt, nicht Working-Tree-Bytes. Da auch lose ZIP-Ordner ohne Git-Kontext gehasht werden, muss die Normalisierung im Tool passieren. Ohne sie hätte dasselbe Addon je nach OS des Spielers eine andere Identität. |
| E-2 | SHA-256 statt SHA-1 | Git-Kompatibilität ist durch E-1 ohnehin aufgegeben. Der Tree-Hash ist der Sicherheits-Anker; SHA-1-Chosen-Prefix-Kollisionen sind praktisch bezahlbar. Messkosten: 0,7 s auf einem vollen 435-MB-Scan. |
| E-3 | Dual-Hash: `git_tree_sha` bleibt als Poll-Cache | Erhält die Möglichkeit, Änderungen über die GitHub-API ohne Clone zu erkennen, ohne den Identitäts-Anker zu schwächen. |
| E-4 | Alle Repos unter `vanilla-wow-dev`, Default-Trust = `vanilla-wow-dev/addon-index` | Konsistent zu Produktname und Domain; ein neutral benanntes Tool mit community-spezifischem Default-Trust wäre inkonsistent. Lädt andere Communities zum Mitkuratieren ein (Ziel 10). |
| E-5 | Frontend: Tailwind + TanStack Table (headless) | Maximale gestalterische Kontrolle für den Tome-Look, kleinstes Bundle, beste Tabellen-Logik. Preis: Eigenbau-Komponenten müssen unter dem 100-%-Coverage-Gate selbst getestet werden — bewusst akzeptiert. |
| E-6 | Hash-Cache-Key `(file_count, max_mtime, total_bytes)` | Der Walk liest die Größe ohnehin (41 ms für 10.364 Dateien); das Tripel fängt zusätzlich „geändert bei erhaltener mtime" ab. |
| E-7 | Keine Git-Bibliothek für den Hash-Pfad | Der Hasher arbeitet direkt auf dem Dateisystem. Damit entfällt die Abwägung `git2` vs `gix` für die Identität und die C-Toolchain-Last im Cross-Build. |
| E-8 | Tree-Hasher als gemeinsame Crate für Client und Collector | Zwei Implementierungen desselben Normalisierungs-Regelwerks würden divergieren; eine Divergenz im Identitäts-Anker wäre der teuerste denkbare Bug. |
| E-9 | Einzelnes `CR` wird **nicht** zu `LF` normalisiert (Korrektur zur ersten Fassung) | Nur CRLF entsteht durch OS-Checkout-Filter. Ein alleinstehendes CR ist echter Datei-Inhalt (z. B. in einem Lua-String-Literal) — es umzuschreiben würde Inhalt verfälschen und wäre für die OS-Unabhängigkeit ohne Nutzen. Entspricht auch Gits eigenem `text=auto`-Verhalten. |
| E-10 | Die Git-Serialisierung wird gegen echtes `git write-tree` kreuzvalidiert, indem im Test SHA-256 gegen SHA-1 getauscht wird | Golden Vectors, die man aus der eigenen Implementierung gewinnt, beweisen nichts. Der Digest-generische Kern erlaubt einen echten externen Abgleich von Objektformat und Sortierregel. |
| E-11 | Maßgeblich ist ausschließlich `<Ordnername>.toc` (case-insensitiv) | Neun Addons im Bestand liefern eine zweite Manifest-Datei (`pfQuest-tbc.toc`, `ShaguTweaks-tbc.toc`, `CallToArms-master.toc`). Der Client lädt nur die namensgleiche. Messbarer Beleg: über *alle* `.toc` gerechnet erscheinen Interface-Werte 20200/20400 (TBC), über die maßgeblichen nur 11000/11100/11200 — ohne die Regel würden Vanilla-Addons als TBC-Addons gelten. |
| E-12 | `.toc`-Parsing schlägt nie fehl; Encoding wird lossy dekodiert | Ein `.toc` liefert reine Anzeige-Metadaten, der Identitäts-Anker ist der Tree-Hash. Ein kaputtes Byte in einem Notes-Feld darf ein Addon nicht unsichtbar machen. 1.12-Clients schrieben in der Locale-Kodierung, Latin-1/GBK ist also jederzeit möglich. |
| E-13 | Der Fingerprint für den Cache-Key lebt in der Tree-Hash-Crate, nicht im Scanner | Er muss dieselben Ausschlussregeln anwenden wie der Hasher. Andernfalls würde die `.git/index`-mtime, die sich bei jedem `git status` ändert, den Cache jedes Developer-Mode-Addons dauerhaft invalidieren. Genau die Divergenz, gegen die E-8 argumentiert. |
| E-14 | Der Cache wird pro Scan **neu aufgebaut** statt ergänzt, und speichert seinen Algorithmus mit | Neuaufbau lässt Einträge gelöschter Addons von selbst verschwinden. Der mitgespeicherte Algorithmus verhindert, dass nach einem Verfahrenswechsel alte Hashes stillschweigend weiterbenutzt werden — ein Cache mit fremdem Algorithmus wird verworfen, nicht gemischt. |
| E-15 | Ein Addon, dessen Hash scheitert, bleibt sichtbar (mit Fehlertext) statt zu verschwinden | Ein Nutzer, der sein Addon nicht in der Liste findet, sucht den Fehler an der falschen Stelle. Nur der Cache-Eintrag entfällt, damit ein Fehlschlag nicht festgeschrieben wird. |
| E-16 | Voll thematisches Design, aber Thema nur in der **Rahmung** | Kopfzeile, Rahmen, Abschnittsköpfe und Badges tragen den Tome-Charakter; Datenzellen bleiben monospace und kontraststark. Eine Liste mit 242 Zeilen ist der Zweck des Bildschirms — Lesbarkeit schlägt dort Atmosphäre. |
| E-17 | Schriften lokal eingebettet statt über ein CDN | Ein Google-Fonts-Request bei jedem Start einer Desktop-App wäre ein Datenabfluss an einen Dritten. SIL OFL 1.1 erlaubt das Mitliefern ausdrücklich; Lizenztexte liegen bei den Dateien. Kosten: 70 KB im Binary, dafür offline-fähig. |
| E-18 | Pergament prozedural (Verläufe + SVG-Rauschen), nicht als Bilddatei | Skaliert verlustfrei auf jeder Auflösung, lässt sich für den Dunkelmodus umfärben statt zu duplizieren, und bläht das Binary nicht auf. |
| E-20 | Die Modus-Spalte benennt nur „Git" und lässt sonst leer | Wir beobachten allein, ob ein `.git/` vorhanden ist. Ob die übrigen Dateien aus einem ZIP, von Hand oder von einem anderen Manager stammen, ist unbekannt — „ZIP" wäre eine Erfindung. Nebeneffekt: 241 von 242 Zeilen bleiben ruhig, die Ausnahme springt ins Auge. |
| E-21 | Der Aktiv-Zustand kommt aus `WTF/…/AddOns.txt`, nicht aus `## DefaultState` | `DefaultState` ist nur der Anfangswert beim ersten Sehen. Messung am realen Bestand: 249 von 259 `.toc` sagen `disabled`, während der Charakter `Zinnober` 34 von 35 Addons aktiv hat. `DefaultState` bleibt im Detailbereich sichtbar, beantwortet die Frage aber nicht. |
| E-22 | „Nie gesehen" ist ein dritter Zustand neben aktiv und aus | Ein Addon, das in `AddOns.txt` fehlt, ist dem Client nie begegnet — das ist etwas anderes als abgeschaltet. Beim Sortieren liegt es zwischen beiden, statt mit einem zu verschmelzen. |
| E-35 | Scrollbereiche zeigen eine **immer gezeichnete** Balken-Spur und reservieren ihren Platz (`scrollbar-gutter: stable`) | Ein Overlay-Balken legte sich über die letzte Spalte und war kaum zu sehen. Erst die stets sichtbare Spur macht überhaupt erkennbar, dass ein Bereich blättert. |
| E-36 | Kein Panel im Panel — verschachtelte Blöcke bekommen `tome-inset` statt eines zweiten Rahmens | Rahmen im Rahmen ergab bis zu drei parallele Goldlinien. Der eingebettete Block trägt jetzt eine getönte Fläche mit einer Kante links. Auch die Doppellinie des Panels selbst (Border plus Outline) ist entfallen. |
| E-37 | Die Charakter-Ansicht lässt Hash, Modus und Dateizahl aus der Tabelle weg | Dort lautet die Frage „was lädt dieser Charakter?", nicht „welche Identität hat dieses Addon?". Die Angaben stehen vollständig im Detailbereich. |
| E-38 | Beim Charakter sortiert der Aktiv-Zustand vor, dann alphabetisch | Beantwortet die Frage der Ansicht ohne einen einzigen Klick. „Nie gesehen" liegt dabei zwischen aktiv und aus. |
| E-32 | Die App füllt das Fenster; gescrollt wird ausschließlich innen | Ein Fenster-Scrollbalken würde Kopfzeile und Seitenleiste mitschieben. Charakterliste und Tabellenrumpf blättern getrennt, der Spaltenkopf klebt oben — bei 242 Zeilen weiß man sonst nach dem ersten Bildschirm nicht mehr, welche Spalte was ist. Waagerecht bleibt am `body` ein `auto` als Notausgang, damit ein zu schmales Fenster den Inhalt nicht unerreichbar abschneidet. |
| E-33 | Fenster-Mindestmaße in `tauri.conf.json` statt nur CSS | 900 × 640. Bei 560 px Höhe blieben nach Kopfzeile, Werkzeugleiste und Veraltet-Hinweis nur zwei Datenzeilen übrig. Das Minimum gehört dorthin, wo das Fenster entsteht — CSS allein kann es nicht erzwingen. |
| E-34 | Panel-Hintergrund deckend statt durchscheinend | Der klebende Spaltenkopf braucht denselben Ton wie das Panel, sonst scheinen die Zeilen darunter durch. Beide beziehen ihn jetzt aus `--panel-bg`. Das Pergament zeigt sich dadurch *um* die Panels statt hindurch — für eine datendichte Ansicht ohnehin ruhiger. |
| E-25 | Seitenleisten-Layout: WoW-Zustand, Addons, dann eine Gruppe „Charaktere" mit einem Eintrag je Charakter | Charaktere sind navigierbare Einheiten mit eigenem Zustand, keine Auswahl in einem Dropdown. Die Zahl daneben zeigt die aktiven Addons und macht den Vergleich ohne Klick möglich. |
| E-26 | Nur die Installation trägt eine Ampel | Ein Punkt an jedem Eintrag hätte bei 11 veralteten Addons zu Dauer-Gelb geführt und wäre damit wertlos. Für Addons und Charaktere steht stattdessen eine Zahl. |
| E-27 | Ein laufender Client zählt als Warnung | WoW schreibt die SavedVariables **beim Beenden** zurück und überschreibt damit alles, was der Manager währenddessen ändert; neue Addons erkennt der Client erst nach Neustart. Beides muss vor einer Änderung gesagt werden, nicht danach. |
| E-28 | Der laufende Client wird über Kommandozeile und Arbeitsverzeichnis erkannt, nicht über den Exe-Pfad | Unter Wine zeigt der Betriebssystem-Prozess auf die Wine-Binary. Die Namensprüfung akzeptiert zusätzlich ein abgeschnittenes `.ex`, weil Linux `comm` auf 15 Zeichen kürzt und `WoW_tweaked_.exe` genau darüber liegt. `WowError.exe` ist ausgenommen — der Absturzmelder läuft gerade dann, wenn das Spiel *nicht* läuft. |
| E-29 | Ein nicht zuordenbarer WoW-Prozess bekommt einen eigenen Zustand statt „läuft hier" | Unter Wine sind Pfade nicht immer auslesbar. Ein ehrliches „unklar" ist besser als eine falsche Behauptung über die eigene Installation. |
| E-30 | Die Charakter-Zustände kommen mit der Charakterliste, nicht mit dem Scan | Die Seitenleiste braucht die Zahl für *alle* Charaktere, nicht nur den gewählten. Nebeneffekt: ein Charakterwechsel ist reine Anzeige und löst keinen erneuten Scan aus. |
| E-31 | Die Aktiv-Spalte erscheint nur in der Charakter-Ansicht | In der reinen Addon-Liste gibt es keinen Charakter-Bezug — eine Spalte voller Striche wäre nur Rauschen. |
| E-24 | Tests verwenden keine laufwerksrelativen Absolutpfade wie `/kein/pfad` | Windows löst sie relativ zum aktuellen Laufwerk auf. Ein Test, der damit einen Schreibfehler provozieren wollte, legte auf dem CI-Runner tatsächlich `D:\kein\pfad` an — die Assertion fiel, und schlimmer: ein anderer Test baute auf das Nichtvorhandensein desselben Verzeichnisses und wäre bei anderer Thread-Reihenfolge mitgefallen. Stattdessen: Pfade unterhalb des Temp-Verzeichnisses, und Schreibfehler über ein Ziel unterhalb einer *Datei* erzwingen. |
| E-23 | Die Interface-Version wird an der Build-Nummer festgemacht, nicht am Hash | Ein gepatchter Client (vanilla-tweaks, Widescreen-Fix) bleibt derselbe Client und lädt dieselben Addons. Die Zuordnung steht in `KNOWN_BUILDS` neben Build und Version. |
| E-19 | TanStack Table **v8** statt des frisch erschienenen v9 | v9 stellt Reaktivität auf TanStack-Store-Atoms und `table.Subscribe` um und verlangt explizites Feature-Opt-in. Keines der neuen Features wird hier gebraucht, die Testbarkeit litte. Die Spaltendefinitionen sind weitgehend portierbar, ein Wechsel bleibt später möglich. |

---

## Offene Punkte

| Thema | Status |
| --- | --- |
| Rust-Git-Bibliothek für Clone/Fetch, falls System-Git fehlt (Windows ohne Git) | offen, erst ab MVP-1 relevant |
| SQLite vs Postgres im Collector | SQLite für MVP, Postgres falls Multi-Curator-Setup wächst |
| Wie die Curator-UI bei Remote-Setup deployed wird (Container? Bare-Metal?) | offen, abhängig vom Hoster |
| Addons, die zur Laufzeit in den eigenen Ordner schreiben (Caches), forken ihren Tree-Hash | offen — Lösungsrichtung: per-Addon-Ignore-Patterns aus dem Index |
| macOS im CI-Matrix (für den NFD-Dateinamen-Fall) | offen; NFD lässt sich vorerst synthetisch auch unter Linux testen |
| Manager-Logging: lokal Datei oder in-app Log-Viewer? | beides, default Datei + UI-Tab |
| Telemetry (anonymisiert): Crash-Reports? | nicht in MVP, später diskutieren |
| Doku-Sprache: Repo-Doku ist Deutsch, UI ist de/en/fr | offen — für ein öffentliches Repo wäre EN als Doku-Quellsprache konsistenter |
| Zuletzt gewählte Ansicht merken (localStorage) | offen — die App startet immer auf dem WoW-Zustand |
| Virtualisierung der Addon-Liste | offen — 242 Zeilen rendert der Webview problemlos; erst nötig, wenn Bestände in die Tausende gehen |
| Visuelle Regressionstests (Screenshots) | offen — das Theme ist derzeit nur manuell geprüft; die Tests decken Verhalten ab, nicht Aussehen |

---

## Referenzen

- Invite-o-matik-Addon als Referenz für das State-Sync-Pattern (`merge.lua`,
  `rebuild-from-archive.lua`, `common.lua`).
- [Tauri v2](https://tauri.app/) — Shell, Updater, Bundling.
- [iroh](https://iroh.computer/) — P2P-Layer (gossip + blobs).
- [TanStack Table](https://tanstack.com/table) — headless Tabellen-Logik.
- [octocrab](https://docs.rs/octocrab/) — GitHub-API-Client für Rust.
- [Instawow](https://github.com/layday/instawow) — Referenz-Manager für
  Classic-Addons.

---

## Nächste Schritte

MVP-0 fertigstellen, in dieser Reihenfolge:

1. ~~**`hash.rs`** — kanonischer Tree-Hash `toa-tree-v1` als eigene Crate.~~ ✅
   Umgesetzt als `crates/tree-hash` (`toa-tree-hash`). 35 Tests, 100 % Line- und
   Function-Coverage, eigener CI-Job auf Ubuntu und Windows. Die Serialisierung
   ist gegen echtes `git write-tree` kreuzvalidiert (siehe E-10). Verifiziert
   gegen den realen Bestand: 259 Ordner, 0 Fehler, deterministisch.
2. ~~**`toc.rs`** — `.toc`-Parser.~~ ✅ Umgesetzt als `crates/toc` (`toa-toc`).
   24 Tests, 100 % Line- und Function-Coverage. Regeln aus einer Erhebung über
   266 reale `.toc`-Dateien abgeleitet (siehe E-11); verifiziert gegen alle 258
   Ordner ohne False Negatives.
3. ~~**`addons.rs`** — Walk über `Interface/AddOns/`.~~ ✅ Umgesetzt in
   `src-tauri/src/addons.rs` samt `scan_addons_command`. Mode-Detection,
   Hash-Cache, `rayon`. 100 % Line- und Function-Coverage. Am realen Bestand:
   242 Addons, 17 übersprungene Ordner, 0 Fehler, **1,70 s kalt → 0,010 s warm
   (175×)**.
4. ~~**UI-Fundament** — Tailwind + TanStack Table.~~ ✅ Tailwind v4 (CSS-first),
   TanStack Table v8, thematisches Design-System in `src/style.css`.
5. ~~**Addon-Liste**~~ ✅ `AddonTable.vue`: sortierbar, durchsuchbar (inkl. Hash
   und Pfad), Detail-Ausklappung, übersprungene Ordner mit Grund. 79
   Frontend-Tests, Lines und Functions bei 100 %.

Damit ist **MVP-0 abgeschlossen**. Als Nächstes MVP-1 (Direct-Git-Install).
