//! Läuft gerade ein WoW-Client?
//!
//! Das ist keine Kosmetik: WoW schreibt die SavedVariables **beim Beenden**
//! zurück. Alles, was der Manager währenddessen im `WTF/`-Ordner ändert, wird
//! dann überschrieben. Und neu installierte Addons erkennt der Client erst nach
//! einem Neustart. Beides muss die Oberfläche sagen können, bevor jemand eine
//! Änderung anstößt, die stillschweigend verpufft.
//!
//! # Warum Kommandozeile und Arbeitsverzeichnis statt des Exe-Pfads
//!
//! Unter Linux läuft der Client über Wine. Der Betriebssystem-Prozess zeigt
//! dann auf die Wine-Binary, nicht auf `WoW.exe` — der Exe-Pfad allein ordnet
//! also nichts zu. Verwertbar sind das Arbeitsverzeichnis (Wine startet im
//! Spielordner) und die Kommandozeile, die den Pfad zur Exe enthält.
//!
//! # Warum die Namensprüfung ein abgeschnittenes `.ex` erlaubt
//!
//! Linux kürzt `comm` auf 15 Zeichen. Aus `WoW_tweaked_.exe` wird dabei
//! `WoW_tweaked_.ex` — eine Prüfung auf `.exe` würde genau die Variante
//! verfehlen, die hier im Ordner liegt.

use std::path::{Path, PathBuf};

use serde::Serialize;

/// Zustand des Spiel-Clients zu einer bestimmten Installation.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ClientState {
    /// Kein WoW-Prozess gefunden.
    NotRunning,
    /// Ein Prozess läuft nachweislich aus dieser Installation.
    RunningHere,
    /// Ein WoW-Prozess läuft, ließ sich aber keiner Installation zuordnen.
    ///
    /// Eigener Zustand statt „läuft hier": Unter Wine sind Pfade nicht immer
    /// auslesbar, und eine falsche Behauptung wäre schlechter als ein ehrliches
    /// „unklar".
    RunningUnknown,
}

/// Was von einem Prozess für die Zuordnung gebraucht wird.
#[derive(Debug, Clone, Default)]
pub(crate) struct ProcessInfo {
    pub name: String,
    pub exe: Option<PathBuf>,
    pub cwd: Option<PathBuf>,
    pub cmdline: Vec<String>,
}

/// Erkennt am Dateinamen, ob es ein WoW-Client ist.
///
/// `WowError.exe` ist der Absturzmelder und zählt ausdrücklich nicht — er läuft
/// gerade *nach* einem Absturz, wenn das Spiel eben nicht mehr läuft.
fn is_client_exe(token: &str) -> bool {
    let file = token
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(token)
        .to_ascii_lowercase();
    file.starts_with("wow")
        && !file.starts_with("wowerror")
        && (file.ends_with(".exe") || file.ends_with(".ex"))
}

/// Vergleichsform eines Pfads: Trennzeichen vereinheitlicht, kleingeschrieben.
///
/// Wine reicht Pfade mal als `Z:\home\…`, mal als `/home/…` durch; Windows ist
/// ohnehin case-insensitiv.
fn path_key(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}

/// Gehört der Prozess nachweislich zu dieser Installation?
fn belongs_to(root: &Path, process: &ProcessInfo) -> bool {
    let root_key = path_key(&root.to_string_lossy());
    if root_key.is_empty() {
        return false;
    }
    let mentions_root = |value: &str| path_key(value).contains(&root_key);

    process
        .cwd
        .as_ref()
        .is_some_and(|cwd| mentions_root(&cwd.to_string_lossy()))
        || process
            .exe
            .as_ref()
            .is_some_and(|exe| mentions_root(&exe.to_string_lossy()))
        || process.cmdline.iter().any(|arg| mentions_root(arg))
}

/// Reine Entscheidung über eine Prozessliste — der testbare Kern.
pub(crate) fn classify(root: &Path, processes: &[ProcessInfo]) -> ClientState {
    let mut seen_any = false;
    for process in processes {
        let is_client =
            is_client_exe(&process.name) || process.cmdline.iter().any(|arg| is_client_exe(arg));
        if !is_client {
            continue;
        }
        if belongs_to(root, process) {
            // Eindeutiger Treffer schlägt jede weitere Vermutung.
            return ClientState::RunningHere;
        }
        seen_any = true;
    }
    if seen_any {
        ClientState::RunningUnknown
    } else {
        ClientState::NotRunning
    }
}

/// Fragt die laufenden Prozesse ab und ordnet sie der Installation zu.
pub fn state(root: &Path) -> ClientState {
    use sysinfo::{ProcessRefreshKind, RefreshKind, System};

    // Nur Prozess-Metadaten auffrischen — Datenträger, Netzwerk und Sensoren
    // interessieren nicht und kosten sonst unnötig Zeit.
    let system = System::new_with_specifics(
        RefreshKind::nothing().with_processes(
            ProcessRefreshKind::nothing()
                .with_cmd(sysinfo::UpdateKind::Always)
                .with_exe(sysinfo::UpdateKind::Always)
                .with_cwd(sysinfo::UpdateKind::Always),
        ),
    );

    let processes: Vec<ProcessInfo> = system
        .processes()
        .values()
        .map(|process| ProcessInfo {
            name: process.name().to_string_lossy().into_owned(),
            exe: process.exe().map(Path::to_path_buf),
            cwd: process.cwd().map(Path::to_path_buf),
            cmdline: process
                .cmd()
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect(),
        })
        .collect();

    classify(root, &processes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = "/home/spieler/games/wow-1.12.1";

    fn process(name: &str) -> ProcessInfo {
        ProcessInfo {
            name: name.to_string(),
            ..Default::default()
        }
    }

    // -------------------------------------------------- Erkennung am Namen

    #[test]
    fn erkennt_die_client_varianten_im_ordner() {
        // Alle drei liegen real in der vermessenen Installation.
        assert!(is_client_exe("WoW.exe"));
        assert!(is_client_exe("WoW_tweaked.exe"));
        assert!(is_client_exe("WoW_tweaked_.exe"));
        assert!(is_client_exe("wow.exe"));
    }

    #[test]
    fn vertraegt_die_15_zeichen_kuerzung_von_comm() {
        // Linux kürzt comm; aus "WoW_tweaked_.exe" wird "WoW_tweaked_.ex".
        assert!(is_client_exe("WoW_tweaked_.ex"));
    }

    #[test]
    fn erkennt_den_client_auch_im_vollen_pfad() {
        assert!(is_client_exe("/home/spieler/games/wow-1.12.1/WoW.exe"));
        assert!(is_client_exe(r"Z:\home\spieler\games\wow-1.12.1\WoW.exe"));
    }

    #[test]
    fn absturzmelder_zaehlt_nicht_als_laufender_client() {
        // WowError.exe läuft gerade dann, wenn das Spiel *nicht* mehr läuft.
        assert!(!is_client_exe("WowError.exe"));
        assert!(!is_client_exe("wowerror.ex"));
    }

    #[test]
    fn andere_programme_im_ordner_zaehlen_nicht() {
        for name in [
            "Launcher.exe",
            "Repair.exe",
            "BackgroundDownloader.exe",
            "NGPatcher.exe",
            "gs-patcher.exe",
            "wine",
            "firefox",
            "",
        ] {
            assert!(!is_client_exe(name), "{name} sollte nicht zählen");
        }
    }

    // ---------------------------------------------------------- Zuordnung

    #[test]
    fn ordnet_ueber_das_arbeitsverzeichnis_zu() {
        // Wine startet im Spielordner — das ist unter Linux der verlässlichste
        // Hinweis, weil der Exe-Pfad auf die Wine-Binary zeigt.
        let running = ProcessInfo {
            name: "WoW.exe".into(),
            exe: Some("/usr/bin/wine64-preloader".into()),
            cwd: Some(ROOT.into()),
            cmdline: vec!["wine".into()],
        };
        assert_eq!(
            classify(Path::new(ROOT), &[running]),
            ClientState::RunningHere
        );
    }

    #[test]
    fn ordnet_ueber_die_kommandozeile_zu() {
        let running = ProcessInfo {
            name: "wine64-preloader".into(),
            cmdline: vec!["wine".into(), format!("{ROOT}/WoW.exe")],
            ..Default::default()
        };
        assert_eq!(
            classify(Path::new(ROOT), &[running]),
            ClientState::RunningHere
        );
    }

    #[test]
    fn ordnet_ueber_wine_pfade_mit_backslashes_zu() {
        let running = ProcessInfo {
            name: "WoW.exe".into(),
            cmdline: vec![r"Z:\home\spieler\games\wow-1.12.1\WoW.exe".into()],
            ..Default::default()
        };
        assert_eq!(
            classify(Path::new(ROOT), &[running]),
            ClientState::RunningHere
        );
    }

    #[test]
    fn ordnet_ueber_den_exe_pfad_zu() {
        // Der Windows-Fall: der Prozess ist die Exe selbst.
        let running = ProcessInfo {
            name: "WoW.exe".into(),
            exe: Some(format!("{ROOT}/WoW.exe").into()),
            ..Default::default()
        };
        assert_eq!(
            classify(Path::new(ROOT), &[running]),
            ClientState::RunningHere
        );
    }

    #[test]
    fn client_aus_einer_anderen_installation_bleibt_unklar() {
        // Zweite Installation daneben: läuft, aber nicht diese hier.
        let running = ProcessInfo {
            name: "WoW.exe".into(),
            cwd: Some("/home/spieler/games/wow-tbc".into()),
            ..Default::default()
        };
        assert_eq!(
            classify(Path::new(ROOT), &[running]),
            ClientState::RunningUnknown
        );
    }

    #[test]
    fn ein_eindeutiger_treffer_schlaegt_die_vermutung() {
        let fremd = ProcessInfo {
            name: "WoW.exe".into(),
            cwd: Some("/woanders".into()),
            ..Default::default()
        };
        let eigen = ProcessInfo {
            name: "WoW.exe".into(),
            cwd: Some(ROOT.into()),
            ..Default::default()
        };
        assert_eq!(
            classify(Path::new(ROOT), &[fremd, eigen]),
            ClientState::RunningHere
        );
    }

    #[test]
    fn ohne_wow_prozess_laeuft_nichts() {
        let processes = [process("firefox"), process("wine"), process("Launcher.exe")];
        assert_eq!(
            classify(Path::new(ROOT), &processes),
            ClientState::NotRunning
        );
        assert_eq!(classify(Path::new(ROOT), &[]), ClientState::NotRunning);
    }

    #[test]
    fn leerer_root_ordnet_nichts_zu() {
        // Sonst würde `contains("")` jeden Prozess als Treffer werten.
        let running = ProcessInfo {
            name: "WoW.exe".into(),
            cwd: Some("/irgendwo".into()),
            ..Default::default()
        };
        assert_eq!(
            classify(Path::new(""), &[running]),
            ClientState::RunningUnknown
        );
    }

    #[test]
    fn state_laeuft_ohne_panik_gegen_echte_prozesse() {
        // Reiner Smoke-Test: das Ergebnis hängt von der Maschine ab, die
        // Entscheidungslogik ist über `classify` deterministisch abgedeckt.
        let _ = state(Path::new(ROOT));
    }

    #[test]
    fn zustand_serialisiert_fuer_das_frontend() {
        for (value, expected) in [
            (ClientState::NotRunning, "\"not-running\""),
            (ClientState::RunningHere, "\"running-here\""),
            (ClientState::RunningUnknown, "\"running-unknown\""),
        ] {
            assert_eq!(serde_json::to_string(&value).unwrap(), expected);
        }
        assert!(format!("{:?}", ClientState::NotRunning).contains("NotRunning"));
        assert_eq!(ClientState::NotRunning, ClientState::NotRunning);
    }

    #[test]
    fn process_info_ist_debugbar_und_klonbar() {
        let info = process("WoW.exe");
        assert!(format!("{:?}", info.clone()).contains("WoW.exe"));
    }
}
