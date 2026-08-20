//! Tauri-IPC-Wrapper (Glue zwischen Frontend und Logik).
//!
//! Bewusst von der Coverage ausgenommen (siehe `--ignore-filename-regex` in
//! `.github/workflows/ci.yml`): Die `#[tauri::command]`-Macros erzeugen versteckte
//! Wrapper, die ausschließlich die Tauri-IPC-Runtime aufruft — headless nicht
//! ausführbar. Die eigentliche Logik liegt in `wow.rs`/`exe.rs`/`relocate.rs` und
//! ist dort zu 100 % getestet; hier bleibt nur triviale Delegation bzw. der nicht
//! testbare Seiteneffekt (Datei kopieren, Prozess starten).

use std::path::{Path, PathBuf};

use tauri::Manager;

use crate::addons::{scan, HashCache, Scan};
use crate::exe::{inspect_wow_exe, WowExeInfo};
use crate::wow::{addons_dir, detect, Detection};

/// Erkennt die zu verwaltende Installation (Walk-up) + Vorschläge (Registry).
#[tauri::command]
pub fn detect_command() -> Detection {
    detect()
}

/// Analysiert die `WoW.exe` eines WoW-Roots.
#[tauri::command]
pub fn inspect_wow_exe_command(root: String) -> Result<WowExeInfo, String> {
    inspect_wow_exe(Path::new(&root))
}

/// Kopiert den Manager in `target_root` und startet die Kopie. Gibt den Zielpfad
/// zurück; das Schließen dieser (alten) Instanz übernimmt das Frontend.
#[tauri::command]
pub fn relocate_into_command(target_root: String) -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| format!("Eigener Pfad unbekannt: {e}"))?;
    let dest = crate::relocate::plan_relocation(&exe, Path::new(&target_root))?;
    std::fs::copy(&exe, &dest).map_err(|e| format!("Kopieren fehlgeschlagen: {e}"))?;
    std::process::Command::new(&dest)
        .spawn()
        .map_err(|e| format!("Neustart fehlgeschlagen: {e}"))?;
    Ok(dest.to_string_lossy().into_owned())
}

/// Cache-Datei pro WoW-Root.
///
/// Der Pfad wird über den Root-Pfad geschlüsselt, damit mehrere Installationen
/// sich nicht gegenseitig den Cache zerschießen. Die Datei liegt im
/// App-Config-Verzeichnis statt neben dem Binary, weil das auch dann schreibbar
/// ist, wenn WoW unter `C:\Program Files` liegt.
fn cache_path(app: &tauri::AppHandle, root: &Path) -> Result<PathBuf, String> {
    use sha1::{Digest, Sha1};
    let base = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Konfigurationsverzeichnis unbekannt: {e}"))?;
    let key = hex::encode(Sha1::digest(root.to_string_lossy().as_bytes()));
    Ok(base
        .join("cache")
        .join("tree-hashes")
        .join(format!("{}.json", &key[..16])))
}

/// Scannt `Interface/AddOns/` des angegebenen WoW-Roots.
#[tauri::command]
pub fn scan_addons_command(app: tauri::AppHandle, root: String) -> Result<Scan, String> {
    let root = Path::new(&root);
    let dir =
        addons_dir(root).ok_or_else(|| format!("Kein Interface/AddOns in {}", root.display()))?;

    let path = cache_path(&app, root)?;
    let mut cache = HashCache::load(&path);
    let result = scan(&dir, &mut cache).map_err(|e| format!("Scan fehlgeschlagen: {e}"))?;
    // Ein nicht schreibbarer Cache kostet nur Zeit beim nächsten Start und darf
    // ein ansonsten erfolgreiches Ergebnis nicht verwerfen.
    let _ = cache.save(&path);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_wrapper_delegates() {
        // Reine Verdrahtungsprüfung — zählt nicht zur Coverage.
        let _ = detect_command();
    }

    #[test]
    fn inspect_wrapper_errors_on_missing_exe() {
        assert!(inspect_wow_exe_command("/definitiv/kein/pfad".into()).is_err());
    }

    #[test]
    fn relocate_wrapper_rejects_invalid_target() {
        assert!(relocate_into_command("/definitiv/kein/wow".into()).is_err());
    }
}
