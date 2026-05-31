//! Verschieben (Kopieren) des Managers in einen WoW-Root, um ihn dort zu
//! verankern. Die reine Entscheidungslogik (validieren, Zielpfad berechnen,
//! Bundle/bereits-verankert erkennen) liegt hier und ist deterministisch
//! getestet; das eigentliche Kopieren + Neustart liegt in `commands.rs`.

use std::path::{Path, PathBuf};

/// Prüft die Voraussetzungen und berechnet den Zielpfad für die Manager-Kopie
/// in `target_root`. Gibt eine menschenlesbare Fehlermeldung zurück, wenn das
/// Verschieben nicht möglich/sinnvoll ist.
pub fn plan_relocation(current_exe: &Path, target_root: &Path) -> Result<PathBuf, String> {
    // macOS-App-Bundles sind Ordner, keine Einzeldatei → nicht automatisch verschiebbar.
    if is_app_bundle(current_exe) {
        return Err(
            "App-Bundles können nicht automatisch verschoben werden — bitte den \
             Manager manuell in den WoW-Ordner kopieren."
                .into(),
        );
    }
    // Ziel muss eine gültige WoW-1.12.1-Installation sein.
    if crate::wow::inspect_root(target_root, "manual").is_none() {
        return Err("Zielordner ist keine gültige WoW-1.12.1-Installation.".into());
    }
    // Liegt der Manager bereits in diesem Ordner, ist nichts zu tun.
    if current_exe.starts_with(target_root) {
        return Err("Der Manager liegt bereits in diesem Ordner.".into());
    }
    let file_name = current_exe
        .file_name()
        .ok_or_else(|| "Eigener Programmpfad nicht bestimmbar.".to_string())?;
    Ok(target_root.join(file_name))
}

/// Erkennt einen Pfad innerhalb eines macOS-App-Bundles (`.../X.app/...`).
fn is_app_bundle(exe: &Path) -> bool {
    exe.components().any(|c| {
        c.as_os_str()
            .to_string_lossy()
            .to_ascii_lowercase()
            .ends_with(".app")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Minimaler gültiger WoW-Root im Temp-Verzeichnis.
    fn fake_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("toa-reloc-{}-{}", std::process::id(), label));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("Data")).unwrap();
        fs::create_dir_all(root.join("Interface")).unwrap();
        fs::write(root.join("WoW.exe"), b"x").unwrap();
        fs::write(root.join("Data").join("base.MPQ"), b"x").unwrap();
        root
    }

    #[test]
    fn plans_destination_for_valid_target() {
        let root = fake_root("ok");
        let exe = Path::new("/home/user/Downloads/tome-of-addons");
        let dest = plan_relocation(exe, &root).unwrap();
        assert_eq!(dest, root.join("tome-of-addons"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_invalid_target() {
        let dir = std::env::temp_dir().join(format!("toa-reloc-{}-bad", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let err = plan_relocation(Path::new("/x/tome"), &dir).unwrap_err();
        assert!(err.contains("keine gültige"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_when_already_inside_target() {
        let root = fake_root("inside");
        let exe = root.join("tome-of-addons");
        let err = plan_relocation(&exe, &root).unwrap_err();
        assert!(err.contains("bereits in diesem Ordner"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_app_bundle() {
        let root = fake_root("bundle");
        let exe = Path::new("/Applications/Tome of Addons.app/Contents/MacOS/tome");
        let err = plan_relocation(exe, &root).unwrap_err();
        assert!(err.contains("App-Bundle"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn errors_when_exe_has_no_filename() {
        let root = fake_root("noname");
        // Root-Pfad "/" hat keinen Dateinamen.
        let err = plan_relocation(Path::new("/"), &root).unwrap_err();
        assert!(err.contains("nicht bestimmbar"));
        let _ = fs::remove_dir_all(&root);
    }
}
