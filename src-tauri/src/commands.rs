//! Tauri-IPC-Wrapper (Glue zwischen Frontend und Logik).
//!
//! Bewusst von der Coverage ausgenommen (siehe `--ignore-filename-regex` in
//! `.github/workflows/ci.yml`): Die `#[tauri::command]`-Macros erzeugen versteckte
//! Wrapper, die ausschließlich die Tauri-IPC-Runtime aufruft — headless nicht
//! ausführbar. Die eigentliche Logik liegt in `wow.rs`/`exe.rs` und ist dort
//! zu 100 % getestet; hier bleibt nur triviale Delegation.

use std::path::Path;

use crate::exe::{inspect_wow_exe, WowExeInfo};
use crate::wow::{detect_wow_roots, WowRoot};

/// Gibt die Liste validierter WoW-Roots an das Frontend.
#[tauri::command]
pub fn detect_wow_roots_command() -> Vec<WowRoot> {
    detect_wow_roots()
}

/// Analysiert die `WoW.exe` eines WoW-Roots.
#[tauri::command]
pub fn inspect_wow_exe_command(root: String) -> Result<WowExeInfo, String> {
    inspect_wow_exe(Path::new(&root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_wrapper_delegates() {
        // Reine Verdrahtungsprüfung — zählt nicht zur Coverage, fängt aber
        // versehentliches Aufruf-Vertauschen ab.
        assert_eq!(detect_wow_roots_command().len(), detect_wow_roots().len());
    }

    #[test]
    fn inspect_wrapper_errors_on_missing_exe() {
        assert!(inspect_wow_exe_command("/definitiv/kein/pfad".into()).is_err());
    }
}
