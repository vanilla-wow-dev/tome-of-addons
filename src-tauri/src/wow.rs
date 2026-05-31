//! WoW-1.12.1-Installations-Erkennung.
//!
//! Ein Verzeichnis gilt als valider WoW-Root, wenn es alle drei Marker erfüllt:
//!   1. `WoW.exe` (case-insensitiv) im Root,
//!   2. mindestens eine `*.MPQ`-Datei im `Data/`-Unterordner,
//!   3. ein `Interface/`-Verzeichnis (`Interface/AddOns/` ist optional/anlegbar).
//!
//! Fund-Strategien (Kandidaten werden anschließend alle durch denselben
//! Validator geprüft):
//!   - Walk-up vom eigenen Binary (Portable-im-WoW-Ordner-Fall).
//!   - Windows-Registry (Manager liegt z. B. auf dem Desktop, WoW unter C:\games\WoW).

use std::path::{Path, PathBuf};

use serde::Serialize;

/// Ein erkannter, validierter WoW-Root samt Detail-Marker und Fund-Methode.
#[derive(Serialize, Clone, Debug)]
pub struct WowRoot {
    /// Absoluter Pfad zum WoW-Root.
    pub path: String,
    /// `WoW.exe` vorhanden (immer true bei einem zurückgegebenen Root).
    pub has_exe: bool,
    /// Mindestens eine `*.MPQ` in `Data/` (immer true bei einem zurückgegebenen Root).
    pub has_mpq: bool,
    /// `Interface/` vorhanden (immer true bei einem zurückgegebenen Root).
    pub has_interface: bool,
    /// `Interface/AddOns/` vorhanden — kann bei frischer Installation fehlen.
    pub has_addons: bool,
    /// Wie dieser Kandidat gefunden wurde: "walkup" | "registry".
    pub method: String,
}

/// Maximale Anzahl Ebenen, die der Walk-up nach oben prüft.
/// Großzügig, damit auch der Dev-Modus (Binary unter `src-tauri/target/debug/`)
/// den umgebenden WoW-Ordner findet.
const WALKUP_MAX_DEPTH: usize = 10;

/// Findet ein Kind von `dir` mit dem gegebenen Namen — case-insensitiv.
///
/// Wichtig für Wine/Linux, wo `Interface` vs. `interface` real unterschiedliche
/// Dateien wären. Gibt den tatsächlichen Pfad zurück (mit Original-Schreibweise).
pub(crate) fn find_child(dir: &Path, name: &str) -> Option<PathBuf> {
    let target = name.to_ascii_lowercase();
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().to_ascii_lowercase() == target {
            return Some(entry.path());
        }
    }
    None
}

/// Prüft, ob `dir` (case-insensitiv) eine Datei namens `WoW.exe` enthält.
fn has_wow_exe(dir: &Path) -> bool {
    find_child(dir, "wow.exe")
        .map(|p| p.is_file())
        .unwrap_or(false)
}

/// Prüft, ob `Data/` mindestens eine `*.MPQ`-Datei enthält (Endung case-insensitiv).
fn has_mpq_in_data(dir: &Path) -> bool {
    let Some(data) = find_child(dir, "data") else {
        return false;
    };
    if !data.is_dir() {
        return false;
    }
    // `read_dir`-Fehler (z. B. unlesbares Verzeichnis) ergibt schlicht einen
    // leeren Iterator — kein separater, schwer testbarer Fehlerzweig.
    for entry in std::fs::read_dir(&data).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if name.ends_with(".mpq") {
            return true;
        }
    }
    false
}

/// Validiert einen Kandidaten-Pfad. Gibt `Some(WowRoot)` zurück, wenn alle
/// Pflicht-Marker (Exe + MPQ + Interface) erfüllt sind, sonst `None`.
///
/// `method` beschreibt, über welche Strategie der Kandidat kam.
fn inspect_root(path: &Path, method: &str) -> Option<WowRoot> {
    if !path.is_dir() {
        return None;
    }

    let has_exe = has_wow_exe(path);
    let has_mpq = has_mpq_in_data(path);
    let interface = find_child(path, "interface").filter(|p| p.is_dir());
    let has_interface = interface.is_some();

    // Pflicht-Marker: alle drei müssen vorhanden sein.
    if !(has_exe && has_mpq && has_interface) {
        return None;
    }

    let has_addons = interface
        .as_deref()
        .and_then(|i| find_child(i, "addons"))
        .map(|p| p.is_dir())
        .unwrap_or(false);

    Some(WowRoot {
        path: path.to_string_lossy().into_owned(),
        has_exe,
        has_mpq,
        has_interface,
        has_addons,
        method: method.to_string(),
    })
}

/// Strategie 1: Walk-up vom eigenen Binary nach oben, jede Ebene validieren.
///
/// Deckt den häufigsten Fall ab: der Manager liegt (portable) im WoW-Ordner
/// oder in einem Unterordner davon. Gibt den ersten Treffer zurück (der
/// nächstgelegene Root nach oben).
fn detect_via_walkup() -> Option<WowRoot> {
    let exe = std::env::current_exe().ok()?;
    walkup_from(exe.parent()?)
}

/// Läuft von `start` (inklusive) bis zu `WALKUP_MAX_DEPTH` Ebenen nach oben und
/// gibt den ersten validen WoW-Root zurück. Ausgelagert für Testbarkeit.
fn walkup_from(start: &Path) -> Option<WowRoot> {
    for (depth, dir) in start.ancestors().enumerate() {
        if depth > WALKUP_MAX_DEPTH {
            break;
        }
        if let Some(root) = inspect_root(dir, "walkup") {
            return Some(root);
        }
    }
    None
}

/// Strategie 2: Windows-Registry nach dem Blizzard-Installationspfad abfragen.
///
/// Deckt das Szenario ab, in dem der Manager z. B. im Download-Ordner liegt,
/// WoW aber unter `C:\games\WoW`. Bei 1.12.1-Privatserver-Installs ist der
/// Schlüssel nicht garantiert vorhanden — deshalb nur eine von mehreren Quellen.
#[cfg(windows)]
fn detect_via_registry() -> Vec<WowRoot> {
    use winreg::enums::*;
    use winreg::RegKey;

    const SUBKEY: &str = r"SOFTWARE\Blizzard Entertainment\World of Warcraft";
    // WoW 1.12.1 ist 32-bit und schreibt auf 64-bit-Windows in die 32-bit-View
    // (physisch unter Wow6432Node). Unser Manager läuft evtl. als 64-bit-Prozess,
    // daher beide Views explizit via Flag abfragen statt Wow6432Node hart zu kodieren
    // (Microsoft-Empfehlung). KEY_WOW64_32KEY/64KEY werden auf 32-bit-Windows ignoriert.
    let views = [KEY_WOW64_32KEY, KEY_WOW64_64KEY];
    let values = ["InstallPath", "GamePath", "UninstallPath"];
    let hives = [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER];

    let mut found = Vec::new();
    for hive in hives {
        let reg = RegKey::predef(hive);
        for view in views {
            let Ok(key) = reg.open_subkey_with_flags(SUBKEY, KEY_READ | view) else {
                continue;
            };
            for value in values {
                let Ok(raw) = key.get_value::<String, _>(value) else {
                    continue;
                };
                let path = PathBuf::from(raw.trim());
                if let Some(root) = inspect_root(&path, "registry") {
                    found.push(root);
                }
            }
        }
    }
    found
}

#[cfg(not(windows))]
fn detect_via_registry() -> Vec<WowRoot> {
    Vec::new()
}

/// Führt alle Fund-Strategien aus und liefert validierte, deduplizierte
/// Kandidaten — Walk-up zuerst (höchste Konfidenz), dann Registry.
pub fn detect_wow_roots() -> Vec<WowRoot> {
    // Verzweigungsfrei (`Option::into_iter`), damit jede Zeile unabhängig davon
    // läuft, ob der Walk-up tatsächlich einen Root findet — sonst wäre die
    // Coverage umgebungsabhängig (Binary innerhalb vs. außerhalb eines WoW-Ordners).
    let mut roots: Vec<WowRoot> = detect_via_walkup().into_iter().collect();
    roots.extend(detect_via_registry());
    dedup_roots(roots)
}

/// Dedupliziert nach normalisiertem Pfad; erster Treffer (= höchste Konfidenz)
/// gewinnt. Ausgelagert für Testbarkeit.
fn dedup_roots(mut roots: Vec<WowRoot>) -> Vec<WowRoot> {
    let mut seen = std::collections::HashSet::new();
    roots.retain(|r| seen.insert(normalize_path_key(&r.path)));
    roots
}

/// Pfad-Schlüssel für Dedup: case-insensitiv + Trailing-Slash entfernt.
fn normalize_path_key(path: &str) -> String {
    path.trim_end_matches(['/', '\\']).to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Legt eine künstliche WoW-Ordnerstruktur in einem eindeutigen Temp-Pfad an.
    fn make_fake_root(label: &str, with_addons: bool) -> PathBuf {
        // Eindeutigkeit ohne Date/Random: Prozess-ID + Label.
        let root = std::env::temp_dir().join(format!("toa-test-{}-{}", std::process::id(), label));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("Data")).unwrap();
        fs::create_dir_all(root.join("Interface")).unwrap();
        if with_addons {
            fs::create_dir_all(root.join("Interface").join("AddOns")).unwrap();
        }
        fs::write(root.join("WoW.exe"), b"stub").unwrap();
        fs::write(root.join("Data").join("base.MPQ"), b"stub").unwrap();
        root
    }

    #[test]
    fn accepts_valid_root_with_all_markers() {
        let root = make_fake_root("valid", true);
        let found = inspect_root(&root, "test").expect("sollte als WoW-Root erkannt werden");
        assert!(found.has_exe && found.has_mpq && found.has_interface && found.has_addons);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn accepts_root_without_addons_folder() {
        // Frische Installation: AddOns-Ordner existiert noch nicht.
        let root = make_fake_root("no-addons", false);
        let found = inspect_root(&root, "test").expect("sollte trotzdem valide sein");
        assert!(found.has_interface && !found.has_addons);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_dir_missing_mpq() {
        let root = make_fake_root("no-mpq", true);
        fs::remove_file(root.join("Data").join("base.MPQ")).unwrap();
        assert!(inspect_root(&root, "test").is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_dir_missing_exe() {
        let root = make_fake_root("no-exe", true);
        fs::remove_file(root.join("WoW.exe")).unwrap();
        assert!(inspect_root(&root, "test").is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn case_insensitive_markers() {
        // Wine/Linux: kleingeschriebene Marker müssen ebenso greifen.
        let root = std::env::temp_dir().join(format!("toa-test-{}-case", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).unwrap();
        fs::create_dir_all(root.join("interface")).unwrap();
        fs::write(root.join("wow.exe"), b"stub").unwrap();
        fs::write(root.join("data").join("patch-3.mpq"), b"stub").unwrap();
        assert!(inspect_root(&root, "test").is_some());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn walkup_finds_root_from_nested_subdir() {
        // Simuliert „Manager liegt in einem Unterordner des WoW-Roots".
        let root = make_fake_root("walkup", true);
        let nested = root.join("tome-of-addons").join("target").join("debug");
        fs::create_dir_all(&nested).unwrap();
        let found = walkup_from(&nested).expect("Root sollte über Walk-up gefunden werden");
        assert_eq!(
            normalize_path_key(&found.path),
            normalize_path_key(&root.to_string_lossy())
        );
        assert_eq!(found.method, "walkup");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn walkup_returns_none_outside_wow() {
        // Ein Verzeichnis ohne WoW-Marker darüber liefert nichts.
        let dir = std::env::temp_dir().join(format!("toa-test-{}-nowow", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        assert!(walkup_from(&dir).is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn walkup_respects_max_depth() {
        // Root liegt weiter oben als WALKUP_MAX_DEPTH Ebenen → kein Treffer.
        let root = make_fake_root("deep", true);
        let mut deep = root.clone();
        for i in 0..(WALKUP_MAX_DEPTH + 3) {
            deep = deep.join(format!("d{i}"));
        }
        fs::create_dir_all(&deep).unwrap();
        assert!(walkup_from(&deep).is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn dedup_keeps_first_of_duplicate_paths() {
        let mk = |path: &str, method: &str| WowRoot {
            path: path.to_string(),
            has_exe: true,
            has_mpq: true,
            has_interface: true,
            has_addons: false,
            method: method.to_string(),
        };
        // Gleicher Pfad, andere Schreibweise + Trailing-Slash → ein Eintrag.
        let roots = vec![
            mk("/games/WoW", "walkup"),
            mk("/games/wow/", "registry"),
            mk("/other/WoW", "registry"),
        ];
        let deduped = dedup_roots(roots);
        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].method, "walkup"); // erster gewinnt
    }

    #[test]
    fn normalize_path_key_is_case_and_slash_insensitive() {
        assert_eq!(
            normalize_path_key("/Games/WoW/"),
            normalize_path_key("/games/wow")
        );
        assert_eq!(
            normalize_path_key(r"C:\Games\WoW\"),
            normalize_path_key(r"c:\games\wow")
        );
    }

    #[test]
    fn detect_wow_roots_runs_without_panicking() {
        // Reiner Integrations-Smoke (Walk-up via current_exe + Registry-No-Op +
        // Dedup): läuft ohne Panic. Das Ergebnis ist umgebungsabhängig und wird
        // hier bewusst NICHT iteriert — jede Iteration wäre env-abhängige Coverage.
        // Validierung und Dedup sind separat deterministisch getestet.
        let _ = detect_wow_roots();
    }

    #[test]
    fn inspect_root_rejects_non_directory() {
        let file = std::env::temp_dir().join(format!("toa-test-{}-file", std::process::id()));
        let _ = fs::remove_file(&file);
        fs::write(&file, b"x").unwrap();
        assert!(inspect_root(&file, "test").is_none());
        let _ = fs::remove_file(&file);
    }

    #[test]
    fn data_as_file_is_not_valid() {
        // "Data" existiert, ist aber eine Datei statt Verzeichnis.
        let root = std::env::temp_dir().join(format!("toa-test-{}-datafile", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("Interface")).unwrap();
        fs::write(root.join("WoW.exe"), b"x").unwrap();
        fs::write(root.join("Data"), b"not a dir").unwrap();
        assert!(inspect_root(&root, "test").is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn data_dir_with_only_non_mpq_is_not_valid() {
        // Loop iteriert über eine Nicht-MPQ-Datei und fällt ohne Treffer durch.
        let root = make_fake_root("nomqp", false);
        fs::remove_file(root.join("Data").join("base.MPQ")).unwrap();
        fs::write(root.join("Data").join("readme.txt"), b"x").unwrap();
        assert!(inspect_root(&root, "test").is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn wow_root_serializes_for_frontend() {
        // Übt die derive(Serialize)-Impl von WowRoot.
        let root = WowRoot {
            path: "/games/WoW".into(),
            has_exe: true,
            has_mpq: true,
            has_interface: true,
            has_addons: false,
            method: "walkup".into(),
        };
        let json = serde_json::to_string(&root).unwrap();
        assert!(json.contains("\"method\":\"walkup\""));
    }

    #[test]
    fn root_without_data_dir_is_not_valid() {
        // Kein "Data"-Eintrag → find_child liefert None.
        let root = std::env::temp_dir().join(format!("toa-test-{}-nodata", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("Interface")).unwrap();
        fs::write(root.join("WoW.exe"), b"x").unwrap();
        assert!(inspect_root(&root, "test").is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn root_without_interface_is_not_valid() {
        // Exe + MPQ vorhanden, aber kein Interface-Verzeichnis.
        let root = std::env::temp_dir().join(format!("toa-test-{}-noiface", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("Data")).unwrap();
        fs::write(root.join("WoW.exe"), b"x").unwrap();
        fs::write(root.join("Data").join("base.MPQ"), b"x").unwrap();
        assert!(inspect_root(&root, "test").is_none());
        let _ = fs::remove_dir_all(&root);
    }
}
