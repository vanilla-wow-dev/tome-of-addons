//! Lesen des `WTF/`-Ordners: welche Charaktere gibt es, und welche Addons hat
//! der Client für sie tatsächlich aktiviert.
//!
//! **Warum nicht `## DefaultState` aus der `.toc`?** Weil das nur der
//! Anfangszustand beim allerersten Sehen ist, nicht der aktuelle. Im hier
//! vermessenen Bestand steht in 249 von 259 maßgeblichen `.toc`-Dateien
//! `disabled`, während der Charakter `Zinnober` 34 von 35 Addons aktiv hat.
//! Wer wissen will, warum ein Addon im Spiel fehlt, braucht diese Datei.
//!
//! Layout: `WTF/Account/<ACCOUNT>/<Realm>/<Charakter>/AddOns.txt`, darin je
//! Zeile `AddonName: enabled` oder `AddonName: disabled`.
//!
//! Es wird nur gelesen, nie geschrieben. Ein kaputter oder unlesbarer
//! `WTF`-Ordner darf den Addon-Scan nicht verhindern.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// Ein Charakter mit eigener Addon-Auswahl.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Character {
    pub account: String,
    pub realm: String,
    pub name: String,
    /// Absoluter Pfad zur `AddOns.txt` dieses Charakters.
    pub path: String,
    /// Fertige Anzeigeform „Charakter · Realm (Account)".
    ///
    /// Als Feld und nicht als Methode, damit die Schreibweise einmal hier
    /// festgelegt ist und nicht im Frontend nachgebaut werden muss.
    pub label: String,
}

/// Listet alle Charaktere mit einer `AddOns.txt` unterhalb eines WoW-Roots.
///
/// Fehlt `WTF/` ganz, ist das Ergebnis leer statt ein Fehler — eine frische
/// Installation hat den Ordner noch nicht.
pub fn list_characters(root: &Path) -> Vec<Character> {
    let Some(accounts) =
        crate::wow::find_child(root, "wtf").and_then(|wtf| crate::wow::find_child(&wtf, "account"))
    else {
        return Vec::new();
    };

    let mut found = Vec::new();
    for account in subdirs(&accounts) {
        let account_name = file_name(&account);
        for realm in subdirs(&account) {
            let realm_name = file_name(&realm);
            for character in subdirs(&realm) {
                let Some(file) = crate::wow::find_child(&character, "addons.txt") else {
                    continue;
                };
                if !file.is_file() {
                    continue;
                }
                let name = file_name(&character);
                found.push(Character {
                    label: format!("{name} · {realm_name} ({account_name})"),
                    account: account_name.clone(),
                    realm: realm_name.clone(),
                    name,
                    path: file.to_string_lossy().into_owned(),
                });
            }
        }
    }
    // Nach Charakternamen, nicht nach Account: gesucht wird „Zinnober", nicht
    // „RYLON8". Lexikografisch wäre „RYLON20" ohnehin vor „RYLON8" gelandet.
    // `read_dir` garantiert keine Reihenfolge; die Auswahlliste soll stabil sein.
    found.sort_by(|a, b| (&a.name, &a.realm, &a.account).cmp(&(&b.name, &b.realm, &b.account)));
    found
}

/// Liest `AddOns.txt` und liefert je Addon-ID, ob es aktiviert ist.
///
/// Die Schlüssel sind kleingeschrieben, weil der Client die Schreibweise nicht
/// zwingend wie den Ordnernamen führt. Unverständliche Zeilen werden
/// übersprungen statt zu einem Fehler zu führen.
pub fn read_states(path: &Path) -> std::io::Result<HashMap<String, bool>> {
    let raw = std::fs::read(path)?;
    let text = String::from_utf8_lossy(&raw);
    let mut states = HashMap::new();
    for line in text.split('\n') {
        let Some((name, value)) = line.trim().split_once(':') else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let enabled = match value.trim().to_ascii_lowercase().as_str() {
            "enabled" => true,
            "disabled" => false,
            // Alles andere ist keine Zustandszeile.
            _ => continue,
        };
        states.insert(name.to_ascii_lowercase(), enabled);
    }
    Ok(states)
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|entry| entry.path())
        .collect();
    out.sort();
    out
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TempRoot {
        root: PathBuf,
    }

    impl TempRoot {
        fn new(label: &str) -> Self {
            let id = COUNTER.fetch_add(1, Ordering::SeqCst);
            let root = std::env::temp_dir().join(format!(
                "toa-wtf-{}-{}-{}",
                std::process::id(),
                id,
                label
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            TempRoot { root }
        }

        /// Legt `WTF/Account/<acc>/<realm>/<char>/AddOns.txt` an.
        fn character(&self, account: &str, realm: &str, name: &str, body: &str) -> PathBuf {
            let dir = self
                .root
                .join("WTF")
                .join("Account")
                .join(account)
                .join(realm)
                .join(name);
            fs::create_dir_all(&dir).unwrap();
            let file = dir.join("AddOns.txt");
            fs::write(&file, body).unwrap();
            file
        }

        fn path(&self) -> &Path {
            &self.root
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn findet_charaktere_ueber_account_und_realm() {
        let tree = TempRoot::new("list");
        tree.character(
            "RYLON8",
            "NostalGeek 1.12",
            "Zinnober",
            "pfQuest: enabled\n",
        );
        tree.character(
            "RYLON8",
            "NostalGeek 1.12",
            "Haensel",
            "pfQuest: disabled\n",
        );
        tree.character(
            "RYLON20",
            "NostalGeek 1.12",
            "Cinderellae",
            "pfUI: enabled\n",
        );

        let chars = list_characters(tree.path());
        assert_eq!(chars.len(), 3);
        // Nach Charakternamen sortiert — danach sucht der Nutzer.
        assert_eq!(
            chars.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
            ["Cinderellae", "Haensel", "Zinnober"]
        );
        assert_eq!(chars[1].account, "RYLON8");
        assert_eq!(chars[1].realm, "NostalGeek 1.12");
        assert!(chars[1].path.ends_with("AddOns.txt"));
        assert_eq!(chars[1].label, "Haensel · NostalGeek 1.12 (RYLON8)");
    }

    #[test]
    fn charakterordner_ohne_addons_txt_zaehlt_nicht() {
        let tree = TempRoot::new("no-file");
        tree.character("A", "R", "MitDatei", "x: enabled\n");
        fs::create_dir_all(tree.path().join("WTF/Account/A/R/OhneDatei")).unwrap();
        let chars = list_characters(tree.path());
        assert_eq!(chars.len(), 1);
        assert_eq!(chars[0].name, "MitDatei");
    }

    #[test]
    fn addons_txt_als_verzeichnis_zaehlt_nicht() {
        let tree = TempRoot::new("dir-not-file");
        fs::create_dir_all(tree.path().join("WTF/Account/A/R/C/AddOns.txt")).unwrap();
        assert!(list_characters(tree.path()).is_empty());
    }

    #[test]
    fn fehlender_wtf_ordner_ist_kein_fehler() {
        // Frische Installation: WTF existiert noch nicht.
        let tree = TempRoot::new("no-wtf");
        assert!(list_characters(tree.path()).is_empty());

        // WTF da, aber ohne Account-Unterordner.
        fs::create_dir_all(tree.path().join("WTF")).unwrap();
        assert!(list_characters(tree.path()).is_empty());
    }

    #[test]
    fn wtf_wird_case_insensitiv_gefunden() {
        // Unter Wine/Linux schreibt der Client teils klein.
        let tree = TempRoot::new("case");
        let dir = tree
            .path()
            .join("wtf")
            .join("account")
            .join("A")
            .join("R")
            .join("C");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("addons.txt"), "pfUI: enabled\n").unwrap();
        assert_eq!(list_characters(tree.path()).len(), 1);
    }

    #[test]
    fn liest_zustaende_case_insensitiv() {
        let tree = TempRoot::new("states");
        // Reale Datei: CRLF, gemischte Schreibweise.
        let file = tree.character(
            "A",
            "R",
            "C",
            "EQL3: enabled\r\nChatLog: disabled\r\npfQuest: ENABLED\r\n",
        );
        let states = read_states(&file).unwrap();
        assert_eq!(states.get("eql3"), Some(&true));
        assert_eq!(states.get("chatlog"), Some(&false));
        assert_eq!(states.get("pfquest"), Some(&true));
        assert_eq!(states.len(), 3);
    }

    #[test]
    fn ueberspringt_zeilen_die_keine_zustaende_sind() {
        let tree = TempRoot::new("junk");
        let file = tree.character(
            "A",
            "R",
            "C",
            "\n# Kommentar\nOhneDoppelpunkt\n: leerer Name\nAddon: vielleicht\nGut: enabled\n",
        );
        let states = read_states(&file).unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(states.get("gut"), Some(&true));
    }

    #[test]
    fn fehlende_datei_meldet_io_fehler() {
        assert!(read_states(Path::new("/definitiv/kein/pfad")).is_err());
    }

    #[test]
    fn character_serialisiert_fuer_das_frontend() {
        let character = Character {
            account: "A".into(),
            realm: "R".into(),
            name: "C".into(),
            path: "/x/AddOns.txt".into(),
            label: "C · R (A)".into(),
        };
        let json = serde_json::to_string(&character).unwrap();
        assert!(json.contains("\"name\":\"C\""));
        assert_eq!(character.clone(), character);
        assert!(format!("{character:?}").contains("Character"));
    }
}
