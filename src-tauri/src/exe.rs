//! Analyse der `WoW.exe`: Build-Nummer, Build-Datum, Größe, Hashes und
//! Abgleich gegen eine Tabelle bekannter offizieller Builds.
//!
//! Identitäts-Quellen, vom Zuverlässigsten zum Schwächsten:
//!   1. **SHA-1/MD5 des Binaries** — exakte Identität. Match gegen die offizielle
//!      Referenz (anzz1/wow-client-checksums) beweist „unverändert".
//!   2. **Eingebetteter Build-String** `WoW [Release] Build NNNN (Datum)` — sagt,
//!      welchen Build der Code *behauptet* zu sein (kann bei Mods abweichen vom
//!      Versions-Resource).
//!
//! Beispiel aus der Praxis: ein `vanilla-tweaks`-gepatchtes Binary meldet
//! denselben Build-String wie das Original, hat aber einen anderen Hash → wird
//! korrekt als „modifiziert" erkannt.

use std::path::Path;

use md5::Md5;
use serde::Serialize;
use sha1::{Digest, Sha1};

/// Ein bekannter, offizieller `WoW.exe`-Build (Referenzwerte zum Abgleich).
struct KnownBuild {
    /// Build-Nummer wie im eingebetteten String.
    build: u32,
    /// Anzeige-Version, z. B. "1.12.1".
    version: &'static str,
    /// Locale der Referenz, z. B. "enUS".
    locale: &'static str,
    /// SHA-1 der offiziellen, unveränderten `WoW.exe` (lowercase hex).
    sha1: &'static str,
}

/// Tabelle bekannter offizieller `WoW.exe`-Hashes.
///
/// Quelle: <https://github.com/anzz1/wow-client-checksums> (original & unmodified).
/// Bewusst klein gehalten und erweiterbar — pro verifiziertem Build/Locale
/// eine Zeile.
const KNOWN_BUILDS: &[KnownBuild] = &[KnownBuild {
    build: 5875,
    version: "1.12.1",
    locale: "enUS",
    sha1: "893def24f703fd18c1514d31b92f00e616d8375f",
}];

/// Verdikt des Identitäts-Abgleichs.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum ExeIdentity {
    /// Hash matcht eine offizielle Referenz exakt.
    Official { version: String, locale: String },
    /// Build-Nummer ist als offiziell bekannt, aber der Hash weicht ab
    /// (gepatcht, z. B. vanilla-tweaks, no-CD, widescreen).
    Modified { claims_version: String },
    /// Build-Nummer ist in keiner Referenz bekannt (Community-Recompile o. Ä.).
    UnknownBuild,
    /// Kein Build-String und kein Hash-Match — kein erkennbarer WoW-Client.
    Unknown,
}

/// Vollständige Analyse einer `WoW.exe`.
#[derive(Serialize, Clone, Debug)]
pub struct WowExeInfo {
    /// Absoluter Pfad zur analysierten Datei.
    pub path: String,
    /// Dateigröße in Bytes.
    pub size_bytes: u64,
    /// Build-Nummer aus dem eingebetteten String (z. B. 5875), falls gefunden.
    pub build: Option<u32>,
    /// Build-Datum aus dem eingebetteten String (z. B. "Sep 19 2006 20:32:39").
    pub build_date: Option<String>,
    /// SHA-1 der Datei (lowercase hex).
    pub sha1: String,
    /// MD5 der Datei (lowercase hex).
    pub md5: String,
    /// Ergebnis des Abgleichs gegen die Referenz-Tabelle.
    pub identity: ExeIdentity,
}

/// Sucht das erste Vorkommen von `needle` in `haystack`.
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Extrahiert Build-Nummer und (optional) Build-Datum aus dem eingebetteten
/// String `... Build NNNN (Datum)`.
fn parse_build_string(bytes: &[u8]) -> (Option<u32>, Option<String>) {
    let needle = b"Build ";
    // Es gibt mehrere "Build "-Vorkommen; wir nehmen das erste, dem direkt
    // mindestens vier Ziffern folgen (das ist der Versions-Build).
    let mut search_from = 0;
    while let Some(rel) = find_subslice(&bytes[search_from..], needle) {
        let after = search_from + rel + needle.len();
        let mut i = after;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let digits = &bytes[after..i];
        if digits.len() >= 4 {
            let build = std::str::from_utf8(digits)
                .ok()
                .and_then(|s| s.parse().ok());
            // Optionales Datum in Klammern direkt dahinter.
            let mut date = None;
            let mut j = i;
            while j < bytes.len() && bytes[j] == b' ' {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'(' {
                let ds = j + 1;
                let mut k = ds;
                while k < bytes.len() && bytes[k] != b')' && k - ds < 64 {
                    k += 1;
                }
                if k < bytes.len() && bytes[k] == b')' {
                    date = std::str::from_utf8(&bytes[ds..k])
                        .ok()
                        .map(|s| s.trim().to_string());
                }
            }
            return (build, date);
        }
        search_from = after;
    }
    (None, None)
}

/// Bestimmt das Identitäts-Verdikt aus Hash und Build-Nummer.
fn classify(sha1: &str, build: Option<u32>) -> ExeIdentity {
    // 1. Exakter Hash-Match gegen offizielle Referenz?
    if let Some(known) = KNOWN_BUILDS.iter().find(|k| k.sha1 == sha1) {
        return ExeIdentity::Official {
            version: known.version.to_string(),
            locale: known.locale.to_string(),
        };
    }
    // 2. Build-Nummer offiziell bekannt, aber Hash weicht ab → modifiziert.
    if let Some(b) = build {
        if let Some(known) = KNOWN_BUILDS.iter().find(|k| k.build == b) {
            return ExeIdentity::Modified {
                claims_version: known.version.to_string(),
            };
        }
        return ExeIdentity::UnknownBuild;
    }
    ExeIdentity::Unknown
}

/// Analysiert die `WoW.exe` unter dem gegebenen WoW-Root.
pub fn inspect_wow_exe(root: &Path) -> Result<WowExeInfo, String> {
    let exe = crate::wow::find_child(root, "wow.exe")
        .ok_or_else(|| "WoW.exe nicht gefunden".to_string())?;
    let bytes = std::fs::read(&exe).map_err(|e| format!("WoW.exe nicht lesbar: {e}"))?;

    let sha1 = hex::encode(Sha1::digest(&bytes));
    let md5 = hex::encode(Md5::digest(&bytes));
    let (build, build_date) = parse_build_string(&bytes);
    let identity = classify(&sha1, build);

    Ok(WowExeInfo {
        path: exe.to_string_lossy().into_owned(),
        size_bytes: bytes.len() as u64,
        build,
        build_date,
        sha1,
        md5,
        identity,
    })
}

/// Tauri-Command: analysiert die `WoW.exe` eines WoW-Roots.
#[tauri::command]
pub fn inspect_wow_exe_command(root: String) -> Result<WowExeInfo, String> {
    inspect_wow_exe(Path::new(&root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_build_string() {
        let data = b"junk WoW [Release] Build 5877 (Sep 19 2006 20:32:39) more junk";
        let (build, date) = parse_build_string(data);
        assert_eq!(build, Some(5877));
        assert_eq!(date.as_deref(), Some("Sep 19 2006 20:32:39"));
    }

    #[test]
    fn parses_build_without_date() {
        let data = b"World of WarCraft (build 5875)";
        // "build " (kleines b) wird vom Needle "Build " nicht getroffen — das
        // ist gewollt; der maßgebliche String ist "[Release] Build NNNN".
        let (build, _) = parse_build_string(data);
        assert_eq!(build, None);
    }

    #[test]
    fn official_hash_is_recognized() {
        let id = classify("893def24f703fd18c1514d31b92f00e616d8375f", Some(5875));
        assert_eq!(
            id,
            ExeIdentity::Official {
                version: "1.12.1".into(),
                locale: "enUS".into()
            }
        );
    }

    #[test]
    fn known_build_wrong_hash_is_modified() {
        // Build 5875 behauptet, aber anderer Hash → modifiziert.
        let id = classify("deadbeef", Some(5875));
        assert_eq!(
            id,
            ExeIdentity::Modified {
                claims_version: "1.12.1".into()
            }
        );
    }

    #[test]
    fn unknown_build_5877_is_unknown_build() {
        // Der reale Fall dieser Installation: community-Build 5877, nicht in der Tabelle.
        let id = classify("abc123", Some(5877));
        assert_eq!(id, ExeIdentity::UnknownBuild);
    }

    /// Legt einen Temp-Root mit einer synthetischen WoW.exe aus `exe_bytes` an.
    fn fake_root_with_exe(label: &str, exe_bytes: &[u8]) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("toa-exe-{}-{}", std::process::id(), label));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("WoW.exe"), exe_bytes).unwrap();
        root
    }

    #[test]
    fn inspect_reads_build_hashes_and_size_end_to_end() {
        let bytes = b"prefix WoW [Release] Build 5877 (Sep 19 2006 20:32:39) suffix-padding-bytes";
        let root = fake_root_with_exe("e2e", bytes);
        let info = inspect_wow_exe(&root).expect("inspect sollte gelingen");
        assert_eq!(info.build, Some(5877));
        assert_eq!(info.build_date.as_deref(), Some("Sep 19 2006 20:32:39"));
        assert_eq!(info.size_bytes, bytes.len() as u64);
        assert_eq!(info.sha1.len(), 40);
        assert_eq!(info.md5.len(), 32);
        assert!(info.sha1.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(info.identity, ExeIdentity::UnknownBuild);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn inspect_flags_known_build_with_foreign_content_as_modified() {
        // Behauptet Build 5875, Inhalt ist aber Müll → Hash weicht ab → modifiziert.
        let bytes = b"WoW [Release] Build 5875 (Sep 19 2006 20:32:39) NOT-THE-REAL-BINARY";
        let root = fake_root_with_exe("modified", bytes);
        let info = inspect_wow_exe(&root).unwrap();
        assert_eq!(
            info.identity,
            ExeIdentity::Modified {
                claims_version: "1.12.1".into()
            }
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn inspect_errors_when_exe_missing() {
        let root = std::env::temp_dir().join(format!("toa-exe-{}-noexe", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert!(inspect_wow_exe(&root).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn build_string_skips_short_digit_runs() {
        // Erstes "Build 12" hat nur 2 Ziffern → übersprungen; "Build 5875" zählt.
        let (build, _) = parse_build_string(b"Build 12 then WoW Build 5875 (x)");
        assert_eq!(build, Some(5875));
    }

    #[test]
    fn build_string_without_parens_has_no_date() {
        let (build, date) = parse_build_string(b"WoW Build 5875 ohne Klammern");
        assert_eq!(build, Some(5875));
        assert_eq!(date, None);
    }

    #[test]
    fn build_string_with_unterminated_parens_has_no_date() {
        // '(' aber kein ')' innerhalb der Grenze → Datum bleibt None.
        let (build, date) = parse_build_string(b"WoW Build 5875 (no closing paren here");
        assert_eq!(build, Some(5875));
        assert_eq!(date, None);
    }

    #[test]
    fn build_string_absent_returns_none() {
        let (build, date) = parse_build_string(b"kein passender Marker hier drin");
        assert_eq!(build, None);
        assert_eq!(date, None);
    }

    #[test]
    fn command_wrapper_inspects_exe() {
        let bytes = b"WoW [Release] Build 5877 (Sep 19 2006 20:32:39)";
        let root = fake_root_with_exe("cmd", bytes);
        let info = inspect_wow_exe_command(root.to_string_lossy().into_owned())
            .expect("command sollte gelingen");
        assert_eq!(info.build, Some(5877));
        assert!(inspect_wow_exe_command("/definitiv/kein/pfad".into()).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
}
