//! Scanner für `Interface/AddOns/`.
//!
//! Führt die beiden Fundament-Crates zusammen: `toa-toc` liest die Metadaten,
//! `toa-tree-hash` liefert die Identität. Dazwischen sitzt ein Hash-Cache, denn
//! ein vollständiger Rehash des hier vermessenen Bestands (259 Ordner, 435 MB)
//! kostet auf einem Rechner ohne SHA-NI rund 3,6 s single-threaded — der
//! Fingerprint-Walk dagegen 41 ms.
//!
//! Grundsatz aus dem Konzept: **die Festplatte ist die Wahrheit.** Persistiert
//! wird ausschließlich der Cache, nie eine Liste installierter Addons.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use toa_tree_hash::Fingerprint;

/// Installationsart eines Addons, am Vorhandensein von `.git/` erkannt.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Entpacktes ZIP ohne lokalen Git-State.
    Consumer,
    /// Git-Checkout, bearbeitbar und pushbar.
    Developer,
}

/// Ein Stück Titel mit eigener Farbe.
#[derive(Serialize, Clone, Debug)]
pub struct TitleSpan {
    pub text: String,
    /// `rrggbb`, oder `None` für die Standardfarbe.
    pub color: Option<String>,
}

/// Ein erkanntes Addon.
#[derive(Serialize, Clone, Debug)]
pub struct Addon {
    /// Ordnername unter `Interface/AddOns/` — der stabile Slug.
    pub id: String,
    pub path: String,
    /// Anzeigename: `## Title` ohne UI-Escapes, sonst der Ordnername.
    ///
    /// Der Fallback ist nicht theoretisch — vier Addons im vermessenen Bestand
    /// haben überhaupt kein `## Title`, und WoW selbst fällt genauso zurück.
    pub title: String,
    /// Derselbe Titel in farbigen Abschnitten. Leer, wenn er aus dem
    /// Ordnernamen stammt — ein Ersatzname hat keine Farben.
    pub title_spans: Vec<TitleSpan>,
    /// Roher `## Title` **inklusive** Farbcodes, ersatzweise der Ordnername.
    ///
    /// Danach sortiert der Client seine Addon-Liste. Für die Anzeige ist er
    /// ungeeignet, für die Reihenfolge unverzichtbar.
    pub title_raw: String,
    /// Reiner `## Version`-String, rein informativ. Bei 143 von 242 Addons
    /// fehlt er ganz; die Identität liefert immer `tree_sha`.
    pub version: Option<String>,
    pub interface: Option<String>,
    pub notes: Option<String>,
    pub author: Option<String>,
    /// Kanonischer `toa-tree-v1`-Hash. `None`, wenn das Hashen fehlschlug.
    pub tree_sha: Option<String>,
    /// Erste 12 Hex-Zeichen für die Anzeige.
    pub tree_sha_short: Option<String>,
    pub mode: Mode,
    /// `## DefaultState` aus der `.toc` — nur der Anfangszustand beim ersten
    /// Sehen, **nicht** ob das Addon aktuell aktiv ist. Dafür siehe `enabled`.
    pub default_state: Option<String>,
    pub file_count: usize,
    pub size_bytes: u64,
    /// Kam der Hash aus dem Cache statt aus einer Neuberechnung?
    pub cached: bool,
    /// Fehlermeldung, falls Fingerprint oder Hash scheiterten.
    pub error: Option<String>,
}

/// Ein Ordner, der kein Addon ist.
#[derive(Serialize, Clone, Debug)]
pub struct Skipped {
    pub id: String,
    pub reason: String,
}

/// Ergebnis eines Scans.
#[derive(Serialize, Clone, Debug, Default)]
pub struct Scan {
    pub addons: Vec<Addon>,
    pub skipped: Vec<Skipped>,
    pub cache_hits: usize,
    pub hashed: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
struct CacheEntry {
    file_count: usize,
    total_bytes: u64,
    max_mtime_nanos: u64,
    tree_sha: String,
}

impl CacheEntry {
    fn matches(&self, fingerprint: &Fingerprint) -> bool {
        self.file_count == fingerprint.file_count
            && self.total_bytes == fingerprint.total_bytes
            && self.max_mtime_nanos == fingerprint.max_mtime_nanos
    }
}

/// Persistenter Hash-Cache, geschlüsselt nach Addon-ID.
///
/// Der Algorithmus-Name wird mitgespeichert: nach einem Wechsel des
/// Hash-Verfahrens dürfen alte Einträge nicht stillschweigend weiterbenutzt
/// werden. Ein Cache mit fremdem Algorithmus wird verworfen, nicht gemischt.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct HashCache {
    algo: String,
    entries: HashMap<String, CacheEntry>,
}

impl HashCache {
    /// Lädt den Cache. Schlägt nie fehl — fehlend, unlesbar, kaputt oder mit
    /// fremdem Algorithmus ergibt schlicht einen leeren Cache. Ein Cache, der
    /// einen Programmstart verhindert, wäre schlimmer als gar keiner.
    pub fn load(path: &Path) -> HashCache {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<HashCache>(&bytes).ok())
            .filter(|cache| cache.algo == toa_tree_hash::ALGO)
            .unwrap_or_default()
    }

    /// Schreibt den Cache, Elternverzeichnis wird bei Bedarf angelegt.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_vec(self)?)
    }
}

/// Scannt ein `Interface/AddOns/`-Verzeichnis.
///
/// Der Cache wird anschließend **neu aufgebaut** statt ergänzt: Einträge zu
/// gelöschten Addons verschwinden damit von selbst, statt die Datei über Jahre
/// wachsen zu lassen.
///
/// `progress` wird nach jedem fertigen Ordner mit `(erledigt, gesamt, id)`
/// gerufen, damit die Oberfläche beim ersten Scan nicht einfriert.
///
/// Der Aktiv-Zustand eines Addons steht bewusst **nicht** hier: er hängt am
/// Charakter, nicht an der Installation, und wird in der Charakter-Ansicht
/// zugeordnet. So bleibt der Scan unabhängig von der Charakterwahl — ein
/// Wechsel kostet keinen erneuten Durchlauf.
pub fn scan_with(
    addons_dir: &Path,
    cache: &mut HashCache,
    progress: &(dyn Fn(usize, usize, &str) + Sync),
) -> std::io::Result<Scan> {
    let mut folders: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(addons_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            folders.push(entry.path());
        }
    }
    // Deterministische Reihenfolge — `read_dir` garantiert keine.
    folders.sort();

    let previous = std::mem::take(&mut cache.entries);
    let total = folders.len();
    let done = std::sync::atomic::AtomicUsize::new(0);
    let outcomes: Vec<Outcome> = folders
        .par_iter()
        .map(|folder| {
            let outcome = scan_one(folder, &previous);
            let count = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            progress(count, total, outcome.id());
            outcome
        })
        .collect();

    let mut scan = Scan::default();
    cache.algo = toa_tree_hash::ALGO.to_string();
    for outcome in outcomes {
        match outcome {
            Outcome::Addon(addon, entry) => {
                if addon.cached {
                    scan.cache_hits += 1;
                } else if addon.tree_sha.is_some() {
                    scan.hashed += 1;
                }
                if let Some(entry) = entry {
                    cache.entries.insert(addon.id.clone(), entry);
                }
                scan.addons.push(*addon);
            }
            Outcome::Skipped(skipped) => scan.skipped.push(skipped),
        }
    }
    Ok(scan)
}

enum Outcome {
    /// Addon plus der Cache-Eintrag, den es hinterlassen soll (keiner bei Fehler).
    Addon(Box<Addon>, Option<CacheEntry>),
    Skipped(Skipped),
}

impl Outcome {
    fn id(&self) -> &str {
        match self {
            Outcome::Addon(addon, _) => &addon.id,
            Outcome::Skipped(skipped) => &skipped.id,
        }
    }
}

fn scan_one(folder: &Path, previous: &HashMap<String, CacheEntry>) -> Outcome {
    let id = folder
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    // Ohne `<Ordnername>.toc` lädt WoW den Ordner nicht — dann ist es kein
    // Addon, sondern eine Backup-Kopie, ein Blizzard-Stub oder eine falsch
    // benannte Installation.
    let toc = match toa_toc::load(folder) {
        Ok(Some(toc)) => toc,
        Ok(None) => {
            return Outcome::Skipped(Skipped {
                id: id.clone(),
                reason: format!("kein {id}.toc — WoW würde diesen Ordner nicht laden"),
            })
        }
        Err(err) => {
            return Outcome::Skipped(Skipped {
                id,
                reason: format!("nicht lesbar: {err}"),
            })
        }
    };

    let mode = if folder.join(".git").is_dir() {
        Mode::Developer
    } else {
        Mode::Consumer
    };

    let display = toc.display_title().filter(|title| !title.is_empty());
    let mut addon = Addon {
        title_raw: match (&display, toc.title()) {
            (Some(_), Some(raw)) => raw.to_string(),
            // Ohne `## Title` zeigt und sortiert der Client den Ordnernamen.
            _ => id.clone(),
        },
        title_spans: if display.is_some() {
            toc.title_segments()
                .into_iter()
                .map(|segment| TitleSpan {
                    text: segment.text,
                    color: segment.color,
                })
                .collect()
        } else {
            Vec::new()
        },
        title: display.unwrap_or_else(|| id.clone()),
        version: toc.version().map(str::to_string),
        interface: toc.interface().map(str::to_string),
        notes: toc.notes().map(str::to_string),
        author: toc.author().map(str::to_string),
        default_state: toc.get("DefaultState").map(str::to_string),
        id: id.clone(),
        path: folder.to_string_lossy().into_owned(),
        tree_sha: None,
        tree_sha_short: None,
        mode,
        file_count: 0,
        size_bytes: 0,
        cached: false,
        error: None,
    };

    let fingerprint = match toa_tree_hash::fingerprint(folder) {
        Ok(fingerprint) => fingerprint,
        Err(err) => {
            addon.error = Some(err.to_string());
            return Outcome::Addon(Box::new(addon), None);
        }
    };
    addon.file_count = fingerprint.file_count;
    addon.size_bytes = fingerprint.total_bytes;

    if let Some(entry) = previous
        .get(&id)
        .filter(|entry| entry.matches(&fingerprint))
    {
        addon.cached = true;
        addon.tree_sha_short = Some(entry.tree_sha[..12].to_string());
        addon.tree_sha = Some(entry.tree_sha.clone());
        return Outcome::Addon(Box::new(addon), Some(entry.clone()));
    }

    match toa_tree_hash::hash_tree(folder) {
        Ok(hash) => {
            let entry = CacheEntry {
                file_count: fingerprint.file_count,
                total_bytes: fingerprint.total_bytes,
                max_mtime_nanos: fingerprint.max_mtime_nanos,
                tree_sha: hash.to_hex(),
            };
            addon.tree_sha_short = Some(hash.short());
            addon.tree_sha = Some(hash.to_hex());
            Outcome::Addon(Box::new(addon), Some(entry))
        }
        Err(err) => {
            addon.error = Some(err.to_string());
            Outcome::Addon(Box::new(addon), None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TempAddons {
        root: PathBuf,
    }

    impl TempAddons {
        fn new(label: &str) -> Self {
            let id = COUNTER.fetch_add(1, Ordering::SeqCst);
            let root = std::env::temp_dir().join(format!(
                "toa-addons-{}-{}-{}",
                std::process::id(),
                id,
                label
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            TempAddons { root }
        }

        /// Legt ein Addon mit passender `.toc` an.
        fn addon(&self, id: &str, toc_body: &str) -> PathBuf {
            let folder = self.root.join(id);
            fs::create_dir_all(&folder).unwrap();
            fs::write(folder.join(format!("{id}.toc")), toc_body).unwrap();
            fs::write(folder.join("core.lua"), b"print('x')\n").unwrap();
            folder
        }

        fn path(&self) -> &Path {
            &self.root
        }

        fn cache_file(&self) -> PathBuf {
            self.root.join("cache").join("tree-hashes.json")
        }
    }

    impl Drop for TempAddons {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    /// Scan ohne Charakter-Zustände und ohne Fortschritts-Empfänger.
    fn scan(dir: &Path, cache: &mut HashCache) -> std::io::Result<Scan> {
        scan_with(dir, cache, &|_, _, _| {})
    }

    /// Pfad, den es garantiert nicht gibt — und der, anders als ein
    /// Absolutpfad wie `/kein/pfad`, unter Windows nicht laufwerksrelativ
    /// aufgelöst und dabei versehentlich angelegt werden kann.
    fn nonexistent(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("toa-nonexistent-{}-{label}", std::process::id()))
    }

    fn find<'a>(scan: &'a Scan, id: &str) -> &'a Addon {
        // `expect` statt `unwrap_or_else(|| panic!(…))`: letzteres erzeugt eine
        // Closure, die bei grünen Tests nie läuft und das 100-%-Gate reißt.
        scan.addons
            .iter()
            .find(|addon| addon.id == id)
            .expect("Addon sollte im Scan sein")
    }

    // ------------------------------------ DefaultState, Aktiv-Zustand, Fortschritt

    #[test]
    fn liest_default_state_aus_der_toc() {
        let tree = TempAddons::new("default-state");
        tree.addon("A", "## Interface: 11200\n## DefaultState: enabled\n");
        tree.addon("B", "## Interface: 11200\n");
        let mut cache = HashCache::default();
        let scan = scan(tree.path(), &mut cache).unwrap();
        assert_eq!(find(&scan, "A").default_state.as_deref(), Some("enabled"));
        assert_eq!(find(&scan, "B").default_state, None);
    }

    #[test]
    fn meldet_fortschritt_fuer_jeden_ordner_genau_einmal() {
        let tree = TempAddons::new("progress");
        for id in ["A", "B", "C"] {
            tree.addon(id, "## Interface: 11200\n");
        }
        // Ohne .toc — muss trotzdem gezählt werden, sonst bliebe der Balken stehen.
        fs::create_dir_all(tree.path().join("KeinAddon")).unwrap();

        let seen = std::sync::Mutex::new(Vec::new());
        let mut cache = HashCache::default();
        scan_with(tree.path(), &mut cache, &|done, total, id| {
            seen.lock().unwrap().push((done, total, id.to_string()));
        })
        .unwrap();

        let seen = seen.into_inner().unwrap();
        assert_eq!(seen.len(), 4);
        assert!(seen.iter().all(|(_, total, _)| *total == 4));
        // Der Zähler läuft lückenlos von 1 bis 4, auch über die Rayon-Threads.
        let mut counts: Vec<usize> = seen.iter().map(|(done, _, _)| *done).collect();
        counts.sort_unstable();
        assert_eq!(counts, [1, 2, 3, 4]);
        let mut ids: Vec<&str> = seen.iter().map(|(_, _, id)| id.as_str()).collect();
        ids.sort_unstable();
        assert_eq!(ids, ["A", "B", "C", "KeinAddon"]);
    }

    // ------------------------------------------------------------ Grundfall

    #[test]
    fn scannt_addons_mit_metadaten_und_hash() {
        let tree = TempAddons::new("basic");
        tree.addon(
            "pfQuest",
            "## Interface: 11200\n## Title: |cff33ffccpf|cffffffffQuest\n\
             ## Version: 1.2.3\n## Author: Shagu\n## Notes: Questhelper\n",
        );
        let mut cache = HashCache::default();
        let scan = scan(tree.path(), &mut cache).unwrap();

        let addon = find(&scan, "pfQuest");
        assert_eq!(addon.title, "pfQuest");
        assert_eq!(addon.version.as_deref(), Some("1.2.3"));
        assert_eq!(addon.interface.as_deref(), Some("11200"));
        assert_eq!(addon.author.as_deref(), Some("Shagu"));
        assert_eq!(addon.notes.as_deref(), Some("Questhelper"));
        assert_eq!(addon.mode, Mode::Consumer);
        assert_eq!(addon.tree_sha.as_ref().unwrap().len(), 64);
        assert_eq!(addon.tree_sha_short.as_ref().unwrap().len(), 12);
        assert!(addon
            .tree_sha
            .as_ref()
            .unwrap()
            .starts_with(addon.tree_sha_short.as_ref().unwrap()));
        assert!(addon.error.is_none());
        assert_eq!(addon.file_count, 2);
        assert!(addon.size_bytes > 0);
        assert_eq!(scan.hashed, 1);
        assert_eq!(scan.cache_hits, 0);
    }

    #[test]
    fn liefert_den_titel_in_farbigen_abschnitten() {
        let tree = TempAddons::new("title-spans");
        tree.addon(
            "pfQuest",
            "## Interface: 11200\n## Title: |cff33ffccpf|cffffffffQuest\n",
        );
        let mut cache = HashCache::default();
        let result = scan(tree.path(), &mut cache).unwrap();
        let addon = find(&result, "pfQuest");
        assert_eq!(addon.title, "pfQuest");
        assert_eq!(addon.title_spans.len(), 2);
        assert_eq!(addon.title_spans[0].text, "pf");
        assert_eq!(addon.title_spans[0].color.as_deref(), Some("33ffcc"));
        assert_eq!(addon.title_spans[1].color.as_deref(), Some("ffffff"));
        assert!(format!("{:?}", addon.title_spans[0]).contains("TitleSpan"));
    }

    #[test]
    fn ein_ersatzname_bekommt_keine_farben() {
        // Der Ordnername ist kein Titel des Autors — ihn einzufärben wäre
        // erfunden.
        let tree = TempAddons::new("no-title-spans");
        tree.addon("CT_BarMod", "## Interface: 11200\n");
        let mut cache = HashCache::default();
        let result = scan(tree.path(), &mut cache).unwrap();
        let addon = find(&result, "CT_BarMod");
        assert_eq!(addon.title, "CT_BarMod");
        assert!(addon.title_spans.is_empty());
    }

    #[test]
    fn bei_doppeltem_titel_gewinnt_die_letzte_angabe() {
        // Realer Fall: Config.toc trug einen Copy-Paste-Rest aus dem
        // Nachbar-Addon in der Zeile darüber.
        let tree = TempAddons::new("duplicate-title");
        tree.addon(
            "Config",
            "## Interface: 11200\n## Title: [mojo] Addons\n## Title: [mojo] Config\n",
        );
        let mut cache = HashCache::default();
        let result = scan(tree.path(), &mut cache).unwrap();
        let addon = find(&result, "Config");
        assert_eq!(addon.title, "[mojo] Config");
    }

    #[test]
    fn titel_faellt_auf_den_ordnernamen_zurueck() {
        // Vier Addons im realen Bestand haben kein `## Title` — WoW zeigt dann
        // ebenfalls den Ordnernamen.
        let tree = TempAddons::new("no-title");
        tree.addon("CT_BarMod", "## Interface: 11200\n");
        tree.addon("Leer", "## Interface: 11200\n## Title:\n");
        let mut cache = HashCache::default();
        let scan = scan(tree.path(), &mut cache).unwrap();
        assert_eq!(find(&scan, "CT_BarMod").title, "CT_BarMod");
        // Ein vorhandener, aber leerer Titel darf ebensowenig durchrutschen.
        assert_eq!(find(&scan, "Leer").title, "Leer");
    }

    #[test]
    fn ergebnis_ist_alphabetisch_stabil() {
        let tree = TempAddons::new("order");
        for id in ["Zed", "Alpha", "Mid"] {
            tree.addon(id, "## Interface: 11200\n");
        }
        let mut cache = HashCache::default();
        let result = scan(tree.path(), &mut cache).unwrap();
        let ids: Vec<&str> = result
            .addons
            .iter()
            .map(|addon| addon.id.as_str())
            .collect();
        assert_eq!(ids, ["Alpha", "Mid", "Zed"]);
    }

    // ------------------------------------------------------ Mode-Erkennung

    #[test]
    fn git_verzeichnis_bedeutet_developer_mode() {
        let tree = TempAddons::new("mode");
        let folder = tree.addon("pfUI", "## Interface: 11200\n");
        fs::create_dir_all(folder.join(".git")).unwrap();
        fs::write(
            folder.join(".git").join("HEAD"),
            b"ref: refs/heads/master\n",
        )
        .unwrap();
        let mut cache = HashCache::default();
        let scan = scan(tree.path(), &mut cache).unwrap();
        assert_eq!(find(&scan, "pfUI").mode, Mode::Developer);
        // `.git` zählt weder in file_count noch in den Hash hinein.
        assert_eq!(find(&scan, "pfUI").file_count, 2);
    }

    // ------------------------------------------- übersprungene Verzeichnisse

    #[test]
    fn ordner_ohne_passende_toc_werden_uebersprungen() {
        let tree = TempAddons::new("skip");
        // Blizzard-Stub: gar keine .toc.
        fs::create_dir_all(tree.path().join("Blizzard_TalentUI")).unwrap();
        // Fehlbenannt: BlizzardPlates/BlizzPlates.toc — existiert real so.
        let wrong = tree.path().join("BlizzardPlates");
        fs::create_dir_all(&wrong).unwrap();
        fs::write(wrong.join("BlizzPlates.toc"), "## Title: X\n").unwrap();
        // Nur eine Variante, keine namensgleiche.
        let variant = tree.path().join("pfQuest");
        fs::create_dir_all(&variant).unwrap();
        fs::write(variant.join("pfQuest-tbc.toc"), "## Title: X\n").unwrap();
        tree.addon("Echt", "## Interface: 11200\n");

        let mut cache = HashCache::default();
        let scan = scan(tree.path(), &mut cache).unwrap();

        assert_eq!(scan.addons.len(), 1);
        let mut skipped: Vec<&str> = scan.skipped.iter().map(|s| s.id.as_str()).collect();
        skipped.sort_unstable();
        assert_eq!(skipped, ["BlizzardPlates", "Blizzard_TalentUI", "pfQuest"]);
        assert!(scan.skipped[0].reason.contains(".toc"));
    }

    #[test]
    fn dateien_im_addons_verzeichnis_werden_ignoriert() {
        let tree = TempAddons::new("stray-file");
        tree.addon("Echt", "## Interface: 11200\n");
        fs::write(tree.path().join("readme.txt"), b"nicht mein Problem").unwrap();
        let mut cache = HashCache::default();
        let scan = scan(tree.path(), &mut cache).unwrap();
        assert_eq!(scan.addons.len(), 1);
        assert!(scan.skipped.is_empty());
    }

    #[test]
    fn fehlendes_addons_verzeichnis_ist_ein_io_fehler() {
        let mut cache = HashCache::default();
        assert!(scan(&nonexistent("scan"), &mut cache).is_err());
    }

    // ------------------------------------------------------- Fehlerbehandlung

    #[test]
    #[cfg(unix)]
    fn unhashbares_addon_bleibt_sichtbar_mit_fehler() {
        // Ein Symlink macht den Ordner unhashbar. Das Addon darf deswegen nicht
        // aus der Liste verschwinden — sonst sucht der Nutzer vergeblich.
        let tree = TempAddons::new("unhashable");
        let folder = tree.addon("Kaputt", "## Interface: 11200\n## Title: Kaputt\n");
        std::os::unix::fs::symlink("core.lua", folder.join("link.lua")).unwrap();

        let mut cache = HashCache::default();
        let scan = scan(tree.path(), &mut cache).unwrap();
        let addon = find(&scan, "Kaputt");
        assert_eq!(addon.title, "Kaputt");
        assert!(addon.tree_sha.is_none());
        assert!(addon.error.as_ref().unwrap().contains("ymlink"));
        assert_eq!(scan.hashed, 0);
        // Ein fehlgeschlagener Hash darf nichts im Cache hinterlassen.
        assert!(cache.entries.is_empty());
    }

    #[test]
    #[cfg(unix)]
    fn unlesbarer_ordner_wird_als_uebersprungen_gemeldet() {
        use std::os::unix::fs::PermissionsExt;
        let tree = TempAddons::new("unreadable");
        let folder = tree.addon("Gesperrt", "## Interface: 11200\n");
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o000)).unwrap();

        let mut cache = HashCache::default();
        let result = scan(tree.path(), &mut cache);
        // Rechte zurücksetzen, damit das Aufräumen im Drop gelingt.
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o755)).unwrap();

        let scan = result.unwrap();
        assert!(scan.addons.is_empty());
        assert_eq!(scan.skipped.len(), 1);
        assert!(scan.skipped[0].reason.contains("nicht lesbar"));
    }

    #[test]
    #[cfg(unix)]
    fn fingerprint_fehler_laesst_das_addon_sichtbar() {
        use std::os::unix::fs::PermissionsExt;
        // Lesbarer Ordner mit .toc, aber unlesbarem Unterverzeichnis: die .toc
        // wird gelesen, der Fingerprint scheitert.
        let tree = TempAddons::new("fp-error");
        let folder = tree.addon("Halbkaputt", "## Interface: 11200\n## Title: Halb\n");
        let sub = folder.join("sub");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("x.lua"), b"x").unwrap();
        fs::set_permissions(&sub, fs::Permissions::from_mode(0o000)).unwrap();

        let mut cache = HashCache::default();
        let result = scan(tree.path(), &mut cache);
        fs::set_permissions(&sub, fs::Permissions::from_mode(0o755)).unwrap();

        let scan = result.unwrap();
        let addon = find(&scan, "Halbkaputt");
        assert_eq!(addon.title, "Halb");
        assert!(addon.tree_sha.is_none());
        assert!(addon.error.is_some());
        assert_eq!(addon.file_count, 0);
    }

    // ----------------------------------------------------------- Hash-Cache

    #[test]
    fn zweiter_scan_trifft_den_cache() {
        let tree = TempAddons::new("cache-hit");
        tree.addon("A", "## Interface: 11200\n");
        tree.addon("B", "## Interface: 11200\n");

        let mut cache = HashCache::default();
        let first = scan(tree.path(), &mut cache).unwrap();
        assert_eq!(first.hashed, 2);
        assert_eq!(first.cache_hits, 0);

        let second = scan(tree.path(), &mut cache).unwrap();
        assert_eq!(second.hashed, 0);
        assert_eq!(second.cache_hits, 2);
        assert!(second.addons.iter().all(|addon| addon.cached));
        // Gleicher Hash wie beim Erstlauf — der Cache lügt nicht.
        assert_eq!(find(&first, "A").tree_sha, find(&second, "A").tree_sha);
        assert_eq!(
            find(&second, "A").tree_sha_short.as_ref().unwrap().len(),
            12
        );
    }

    #[test]
    fn geaenderter_inhalt_invalidiert_den_cache() {
        let tree = TempAddons::new("cache-miss");
        let folder = tree.addon("A", "## Interface: 11200\n");
        let mut cache = HashCache::default();
        let before = scan(tree.path(), &mut cache).unwrap();

        fs::write(folder.join("core.lua"), b"print('etwas ganz anderes')\n").unwrap();
        let after = scan(tree.path(), &mut cache).unwrap();

        assert_eq!(after.hashed, 1);
        assert_eq!(after.cache_hits, 0);
        assert_ne!(find(&before, "A").tree_sha, find(&after, "A").tree_sha);
    }

    #[test]
    fn cache_wird_neu_aufgebaut_statt_zu_wachsen() {
        let tree = TempAddons::new("cache-prune");
        tree.addon("Bleibt", "## Interface: 11200\n");
        let weg = tree.addon("Verschwindet", "## Interface: 11200\n");
        let mut cache = HashCache::default();
        scan(tree.path(), &mut cache).unwrap();
        assert_eq!(cache.entries.len(), 2);

        fs::remove_dir_all(&weg).unwrap();
        scan(tree.path(), &mut cache).unwrap();
        assert_eq!(cache.entries.len(), 1);
    }

    #[test]
    fn cache_ueberlebt_speichern_und_laden() {
        let tree = TempAddons::new("cache-roundtrip");
        tree.addon("A", "## Interface: 11200\n");
        let mut cache = HashCache::default();
        scan(tree.path(), &mut cache).unwrap();

        let file = tree.cache_file();
        cache.save(&file).unwrap();
        let loaded = HashCache::load(&file);
        assert_eq!(loaded, cache);

        let mut loaded = loaded;
        let scan = scan(tree.path(), &mut loaded).unwrap();
        assert_eq!(scan.cache_hits, 1);
    }

    #[test]
    fn kaputter_oder_fehlender_cache_ergibt_einen_leeren() {
        let tree = TempAddons::new("cache-broken");
        assert!(HashCache::load(&tree.cache_file()).entries.is_empty());

        let file = tree.path().join("muell.json");
        fs::write(&file, b"{ das ist kein JSON").unwrap();
        assert!(HashCache::load(&file).entries.is_empty());

        // Gültiges JSON, aber kein Cache-Objekt.
        fs::write(&file, b"[1,2,3]").unwrap();
        assert!(HashCache::load(&file).entries.is_empty());
    }

    #[test]
    fn cache_mit_fremdem_algorithmus_wird_verworfen() {
        // Nach einem Algorithmuswechsel dürfen alte Hashes nicht weiterleben.
        let tree = TempAddons::new("cache-algo");
        tree.addon("A", "## Interface: 11200\n");
        let mut cache = HashCache::default();
        scan(tree.path(), &mut cache).unwrap();

        let file = tree.cache_file();
        cache.save(&file).unwrap();
        let mut raw: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        raw["algo"] = serde_json::Value::String("toa-tree-v0".into());
        fs::write(&file, serde_json::to_vec(&raw).unwrap()).unwrap();

        assert!(HashCache::load(&file).entries.is_empty());
    }

    #[test]
    fn speichern_legt_das_elternverzeichnis_an() {
        let tree = TempAddons::new("cache-mkdir");
        let deep = tree.path().join("a").join("b").join("c.json");
        HashCache::default().save(&deep).unwrap();
        assert!(deep.is_file());
    }

    #[test]
    fn speichern_meldet_io_fehler() {
        // Ziel unterhalb einer *Datei*: das Anlegen des Elternverzeichnisses
        // muss scheitern. Ein Absolutpfad wie "/kein/pfad" taugt dafür nicht —
        // Windows löst ihn laufwerksrelativ auf, legt ihn an, und der Test
        // hinterließe obendrein Verzeichnisse, auf deren Fehlen andere Tests
        // bauen.
        let tree = TempAddons::new("cache-io-error");
        let blocker = tree.path().join("blocker");
        fs::write(&blocker, b"x").unwrap();
        assert!(HashCache::default()
            .save(&blocker.join("sub").join("c.json"))
            .is_err());

        // Leerer Pfad: `parent()` ist `None`, der mkdir-Zweig entfällt, das
        // Schreiben scheitert trotzdem — auf beiden Plattformen.
        assert!(HashCache::default().save(Path::new("")).is_err());
    }

    // --------------------------------------------------------- Serialisierung

    #[test]
    fn serialisiert_fuer_das_frontend() {
        let tree = TempAddons::new("serde");
        let folder = tree.addon("A", "## Interface: 11200\n## Title: A\n");
        fs::create_dir_all(folder.join(".git")).unwrap();
        let mut cache = HashCache::default();
        let scan = scan(tree.path(), &mut cache).unwrap();

        let json = serde_json::to_string(&scan).unwrap();
        assert!(json.contains("\"mode\":\"developer\""));
        assert!(json.contains("\"cache_hits\":0"));
        assert!(json.contains("\"tree_sha_short\""));
        // Debug/Clone werden von der Tauri-Schicht benötigt.
        assert!(format!("{:?}", scan.clone()).contains("Addon"));
        assert_eq!(Mode::Consumer, Mode::Consumer);
        assert!(format!("{:?}", Mode::Developer).contains("Developer"));
    }
}
