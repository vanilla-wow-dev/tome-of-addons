//! Tauri-IPC-Wrapper (Glue zwischen Frontend und Logik).
//!
//! Bewusst von der Coverage ausgenommen (siehe `--ignore-filename-regex` in
//! `.github/workflows/ci.yml`): Die `#[tauri::command]`-Macros erzeugen versteckte
//! Wrapper, die ausschließlich die Tauri-IPC-Runtime aufruft — headless nicht
//! ausführbar. Die eigentliche Logik liegt in `wow.rs`/`exe.rs`/`relocate.rs` und
//! ist dort zu 100 % getestet; hier bleibt nur triviale Delegation bzw. der nicht
//! testbare Seiteneffekt (Datei kopieren, Prozess starten).

use std::path::Path;

use crate::exe::{inspect_wow_exe, WowExeInfo};
use crate::wow::{detect, Detection};

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
