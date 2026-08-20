//! Parser for WoW 1.12.1 addon `.toc` manifests.
//!
//! A `.toc` file is line-oriented and forgiving:
//!
//! ```text
//! ## Interface: 11200
//! ## Title: |cff33ffccpf|cffffffffQuest
//! ## Notes-deDE: Ein leichtgewichtiger Questhelfer
//! ## SavedVariables: pfQuest_questcache
//! #
//! init\data.xml
//! init\enUS.xml
//! ```
//!
//! Lines starting with `##` carry directives, other non-empty lines form the
//! load list, and everything else is a comment. Parsing therefore never fails —
//! a malformed line is simply not a directive.
//!
//! # Behaviour derived from a real corpus
//!
//! The rules below are not guesses; they come from surveying 266 `.toc` files
//! across 259 addon folders of a live 1.12.1 installation:
//!
//! - **Only `<FolderName>.toc` counts.** Nine addons ship a second manifest
//!   (`pfQuest-tbc.toc`, `ShaguTweaks-tbc.toc`, `CallToArms-master.toc`). The
//!   client loads exactly the one matching the folder name, and so does
//!   [`find_toc`] — case-insensitively, because `LevelWiz/LevelWiz.Toc` exists.
//! - **A UTF-8 BOM must be stripped.** Nine files carry one; leaving it in
//!   turns the first key into `\u{feff}Interface`.
//! - **Titles routinely contain UI escapes.** 106 of 259 titles use colour
//!   codes, some without a closing `|r` (`|cff33ffccShagu|cffffffffPlates`).
//!   Use [`Toc::display_title`] for anything user-facing.
//! - **The load list uses backslashes** (`init\data.xml`). [`Toc::files`]
//!   normalizes them to `/`.
//! - **Values are free-form.** Observed versions include `0.47b`, `0.1 DEVEL`
//!   and `GIT`; observed interface values include a trailing space. Nothing is
//!   coerced into a number — per the concept the version string is display
//!   metadata only, never identity.
//! - **Encoding is not guaranteed.** The surveyed corpus was pure UTF-8, but
//!   1.12 clients wrote in the locale encoding, so decoding is lossy rather
//!   than fallible. Metadata is never the identity anchor; the tree hash is.

use std::io;
use std::path::{Path, PathBuf};

/// A parsed `.toc` manifest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Toc {
    directives: Vec<(String, String)>,
    files: Vec<String>,
}

impl Toc {
    /// Parses a manifest. Never fails — unparseable lines are ignored.
    pub fn parse(bytes: &[u8]) -> Toc {
        let text = String::from_utf8_lossy(strip_bom(bytes));
        let mut directives = Vec::new();
        let mut files = Vec::new();

        for raw_line in text.split('\n') {
            let line = raw_line.trim();
            if let Some(rest) = line.strip_prefix("##") {
                // A directive without a colon (a bare `##` separator line is
                // common) carries no value and is dropped.
                if let Some((key, value)) = rest.split_once(':') {
                    directives.push((key.trim().to_string(), value.trim().to_string()));
                }
            } else if line.starts_with('#') || line.is_empty() {
                // Comment or blank.
            } else {
                files.push(line.replace('\\', "/"));
            }
        }

        Toc { directives, files }
    }

    /// Value of a directive, matched case-insensitively. If a key appears more
    /// than once the first occurrence wins.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.directives
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }

    /// Locale-specific value with fallback to the base directive, e.g.
    /// `localized("Title", "deDE")` tries `Title-deDE` and then `Title`.
    pub fn localized(&self, key: &str, locale: &str) -> Option<&str> {
        self.get(&format!("{key}-{locale}"))
            .or_else(|| self.get(key))
    }

    /// Splits a comma-separated directive such as `Dependencies` or
    /// `SavedVariables`. Empty entries are dropped, so `## Dependencies:`
    /// yields an empty list rather than one blank name.
    pub fn list(&self, key: &str) -> Vec<&str> {
        self.get(key)
            .map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Raw title, UI escape sequences included.
    pub fn title(&self) -> Option<&str> {
        self.get("Title")
    }

    /// Title with UI escapes removed and internal whitespace collapsed — use
    /// this for anything user-facing.
    ///
    /// The collapsing matters in practice: authors habitually pad around colour
    /// codes (`XRaidStatus |cff7fff7f -Ace2-|r`), so stripping alone leaves
    /// double spaces in a large share of the 106 colour-coded titles observed.
    pub fn display_title(&self) -> Option<String> {
        self.title()
            .map(|title| collapse_whitespace(&strip_ui_escapes(title)))
    }

    pub fn version(&self) -> Option<&str> {
        self.get("Version")
    }

    pub fn interface(&self) -> Option<&str> {
        self.get("Interface")
    }

    pub fn notes(&self) -> Option<&str> {
        self.get("Notes")
    }

    pub fn author(&self) -> Option<&str> {
        self.get("Author")
    }

    /// All directives in file order, keys and values trimmed.
    pub fn directives(&self) -> &[(String, String)] {
        &self.directives
    }

    /// The load list, with `\` separators normalized to `/`.
    pub fn files(&self) -> &[String] {
        &self.files
    }
}

/// Replaces every run of whitespace with a single space.
fn collapse_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Removes a UTF-8 byte order mark if present.
fn strip_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes)
}

/// Removes WoW UI escape sequences from a display string.
///
/// Handles `||` (literal pipe), `|c` colour starts with up to eight hex digits,
/// `|r` colour resets, `|n` newlines, `|T…|t` textures and `|H…|h text |h`
/// hyperlinks (the link payload is dropped, the visible text kept). Unknown
/// escapes are dropped rather than passed through, and an unterminated sequence
/// at the end of the string is tolerated — colour codes without a closing `|r`
/// occur throughout the real corpus.
pub fn strip_ui_escapes(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(current) = chars.next() {
        if current != '|' {
            out.push(current);
            continue;
        }
        match chars.next() {
            Some('|') => out.push('|'),
            Some('c' | 'C') => {
                for _ in 0..8 {
                    if chars.peek().is_some_and(char::is_ascii_hexdigit) {
                        chars.next();
                    } else {
                        break;
                    }
                }
            }
            Some('n') => out.push('\n'),
            Some('T') => skip_until_escape(&mut chars, 't'),
            Some('H') => skip_until_escape(&mut chars, 'h'),
            // `|r`, the closing `|h` of a hyperlink, unknown escapes, and a
            // trailing lone `|` all contribute nothing.
            _ => {}
        }
    }
    out.trim().to_string()
}

/// Consumes input up to and including `|<terminator>`.
fn skip_until_escape(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, terminator: char) {
    while let Some(current) = chars.next() {
        if current == '|' && chars.peek() == Some(&terminator) {
            chars.next();
            return;
        }
    }
}

/// Locates the authoritative manifest of an addon folder: the file named
/// `<FolderName>.toc`, matched case-insensitively.
///
/// Returns `Ok(None)` when the folder holds no matching manifest — that is a
/// normal condition, not an error. The twelve `Blizzard_*` stub folders of a
/// stock installation have none at all, and a folder carrying only
/// `pfQuest-tbc.toc` is deliberately not a match.
pub fn find_toc(folder: &Path) -> io::Result<Option<PathBuf>> {
    let Some(folder_name) = folder.file_name().and_then(|name| name.to_str()) else {
        return Ok(None);
    };
    let wanted = format!("{folder_name}.toc");
    for entry in std::fs::read_dir(folder)? {
        let entry = entry?;
        if entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.eq_ignore_ascii_case(&wanted))
        {
            return Ok(Some(entry.path()));
        }
    }
    Ok(None)
}

/// Reads and parses the authoritative manifest of an addon folder.
pub fn load(folder: &Path) -> io::Result<Option<Toc>> {
    match find_toc(folder)? {
        Some(path) => Ok(Some(Toc::parse(&std::fs::read(path)?))),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TempDir {
        root: PathBuf,
    }

    impl TempDir {
        fn new(label: &str) -> Self {
            let id = COUNTER.fetch_add(1, Ordering::SeqCst);
            let root = std::env::temp_dir().join(format!(
                "toa-toc-{}-{}-{}",
                std::process::id(),
                id,
                label
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            TempDir { root }
        }

        /// Creates `<root>/<folder>` and returns its path.
        fn addon(&self, folder: &str) -> PathBuf {
            let path = self.root.join(folder);
            fs::create_dir_all(&path).unwrap();
            path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    /// Mirrors the shape of a real manifest: LF endings, mixed directives,
    /// a bare `##`, a plain `#` comment, blank lines, backslash paths.
    const SAMPLE: &[u8] = b"\
## Interface: 11200
## Title: |cff33ffccpf|cffffffffQuest
## Author: Shagu
## Notes: A lightweight Questhelper and Database
## Notes-deDE: Ein leichtgewichtiger Questhelfer
## Version: GIT
## OptionalDeps: pfUI, , Ace2
## SavedVariables: pfQuest_questcache
## Dependencies:
##
# a plain comment

init\\data.xml
init\\enUS.xml
";

    // ------------------------------------------------------------- parsing

    #[test]
    fn parses_directives_and_load_list() {
        let toc = Toc::parse(SAMPLE);
        assert_eq!(toc.interface(), Some("11200"));
        assert_eq!(toc.author(), Some("Shagu"));
        assert_eq!(toc.version(), Some("GIT"));
        assert_eq!(toc.notes(), Some("A lightweight Questhelper and Database"));
        assert_eq!(toc.title(), Some("|cff33ffccpf|cffffffffQuest"));
        assert_eq!(toc.files(), ["init/data.xml", "init/enUS.xml"]);
    }

    #[test]
    fn bare_double_hash_and_comments_are_not_directives() {
        let toc = Toc::parse(SAMPLE);
        // `##`, `# a plain comment` and the blank line contribute nothing.
        let keys: Vec<&str> = toc.directives().iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(
            keys,
            [
                "Interface",
                "Title",
                "Author",
                "Notes",
                "Notes-deDE",
                "Version",
                "OptionalDeps",
                "SavedVariables",
                "Dependencies",
            ]
        );
    }

    #[test]
    fn utf8_bom_is_stripped() {
        // Nine files in the surveyed corpus start with one.
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"## Interface: 11200\n");
        let toc = Toc::parse(&bytes);
        assert_eq!(toc.interface(), Some("11200"));
        assert_eq!(toc.directives()[0].0, "Interface");
    }

    #[test]
    fn crlf_line_endings_are_handled() {
        // 134 of 266 surveyed files use CRLF.
        let unix = Toc::parse(b"## Title: X\n## Version: 1.0\nfile.lua\n");
        let dos = Toc::parse(b"## Title: X\r\n## Version: 1.0\r\nfile.lua\r\n");
        assert_eq!(unix, dos);
        assert_eq!(dos.version(), Some("1.0"));
    }

    #[test]
    fn values_and_keys_are_trimmed() {
        // `## Interface: 11200 ` with a trailing space occurs in the corpus.
        let toc = Toc::parse(b"##   Interface  :   11200   \n");
        assert_eq!(toc.interface(), Some("11200"));
        assert_eq!(toc.directives()[0].0, "Interface");
    }

    #[test]
    fn empty_values_are_preserved_as_empty() {
        let toc = Toc::parse(b"## Dependencies:\n");
        assert_eq!(toc.get("Dependencies"), Some(""));
        assert!(toc.list("Dependencies").is_empty());
    }

    #[test]
    fn lookup_is_case_insensitive_and_first_wins() {
        let toc = Toc::parse(b"## Title: first\n## title: second\n");
        assert_eq!(toc.get("TITLE"), Some("first"));
        assert_eq!(toc.directives().len(), 2);
    }

    #[test]
    fn missing_directives_read_as_none() {
        let toc = Toc::parse(b"");
        assert_eq!(toc.title(), None);
        assert_eq!(toc.version(), None);
        assert_eq!(toc.interface(), None);
        assert_eq!(toc.notes(), None);
        assert_eq!(toc.author(), None);
        assert_eq!(toc.display_title(), None);
        assert!(toc.files().is_empty());
        assert_eq!(toc, Toc::default());
    }

    #[test]
    fn invalid_utf8_is_decoded_lossily_rather_than_failing() {
        let toc = Toc::parse(b"## Title: caf\xE9 latin1\n## Version: 1.0\n");
        assert!(toc.title().unwrap().contains("latin1"));
        assert_eq!(toc.version(), Some("1.0"));
    }

    #[test]
    fn debug_and_clone_are_available() {
        let toc = Toc::parse(b"## Title: X\n");
        assert_eq!(toc.clone(), toc);
        assert!(format!("{toc:?}").contains("Title"));
    }

    // ---------------------------------------------------------------- lists

    #[test]
    fn list_splits_trims_and_drops_blanks() {
        let toc = Toc::parse(SAMPLE);
        assert_eq!(toc.list("OptionalDeps"), ["pfUI", "Ace2"]);
        assert_eq!(toc.list("SavedVariables"), ["pfQuest_questcache"]);
        assert!(toc.list("NoSuchKey").is_empty());
    }

    // ----------------------------------------------------------- localized

    #[test]
    fn localized_prefers_the_locale_then_falls_back() {
        let toc = Toc::parse(SAMPLE);
        assert_eq!(
            toc.localized("Notes", "deDE"),
            Some("Ein leichtgewichtiger Questhelfer")
        );
        assert_eq!(
            toc.localized("Notes", "frFR"),
            Some("A lightweight Questhelper and Database")
        );
        assert_eq!(toc.localized("Nothing", "deDE"), None);
    }

    // ------------------------------------------------------- escape codes

    #[test]
    fn strips_colour_codes_including_unterminated_ones() {
        // Real title from the corpus — note the missing `|r`.
        assert_eq!(
            strip_ui_escapes("|cff33ffccShagu|cffffffffPlates"),
            "ShaguPlates"
        );
        assert_eq!(
            strip_ui_escapes("DPSMate |cFFFF8080-Shino-|r"),
            "DPSMate -Shino-"
        );
        // Faithful: the author padded on both sides of the colour code, so
        // stripping alone leaves two spaces. Collapsing is display's job.
        assert_eq!(
            strip_ui_escapes("XRaidStatus |cff7fff7f -Ace2-|r"),
            "XRaidStatus  -Ace2-"
        );
    }

    #[test]
    fn display_title_collapses_padding_around_colour_codes() {
        let toc = Toc::parse(b"## Title: XRaidStatus |cff7fff7f -Ace2-|r\n");
        assert_eq!(toc.display_title(), Some("XRaidStatus -Ace2-".to_string()));
        // Newline escapes collapse into the single-line display form too.
        let multi = Toc::parse(b"## Title: A|nB\n");
        assert_eq!(multi.display_title(), Some("A B".to_string()));
    }

    #[test]
    fn strips_textures_hyperlinks_and_keeps_link_text() {
        assert_eq!(strip_ui_escapes("a|TInterface\\Icons\\x:16|tb"), "ab");
        assert_eq!(
            strip_ui_escapes("|Hitem:1234:0:0:0|h[Thunderfury]|h"),
            "[Thunderfury]"
        );
    }

    #[test]
    fn handles_pipes_newlines_and_unknown_escapes() {
        assert_eq!(strip_ui_escapes("a||b"), "a|b");
        assert_eq!(strip_ui_escapes("a|nb"), "a\nb");
        assert_eq!(strip_ui_escapes("a|Zb"), "ab");
        // Trailing lone pipe, and a `|c` with fewer than eight hex digits.
        assert_eq!(strip_ui_escapes("abc|"), "abc");
        assert_eq!(strip_ui_escapes("|cffzzTitle"), "zzTitle");
        // Unterminated texture escape must not loop or panic.
        assert_eq!(strip_ui_escapes("a|Tunterminated"), "a");
    }

    #[test]
    fn display_title_strips_and_trims() {
        let toc = Toc::parse(SAMPLE);
        assert_eq!(toc.display_title(), Some("pfQuest".to_string()));
    }

    // ------------------------------------------------------ folder lookup

    #[test]
    fn finds_the_manifest_matching_the_folder_name() {
        let temp = TempDir::new("find");
        let folder = temp.addon("pfQuest");
        // Nine addons in the corpus ship a second manifest; only the one
        // matching the folder name is authoritative.
        fs::write(folder.join("pfQuest-tbc.toc"), b"## Title: tbc\n").unwrap();
        fs::write(folder.join("pfQuest.toc"), b"## Title: vanilla\n").unwrap();
        let found = find_toc(&folder).unwrap().unwrap();
        assert_eq!(found.file_name().unwrap(), "pfQuest.toc");
    }

    #[test]
    fn folder_lookup_is_case_insensitive() {
        // `LevelWiz/LevelWiz.Toc` exists in the corpus.
        let temp = TempDir::new("case");
        let folder = temp.addon("LevelWiz");
        fs::write(folder.join("LevelWiz.Toc"), b"## Title: LevelWiz\n").unwrap();
        assert!(find_toc(&folder).unwrap().is_some());
    }

    #[test]
    fn a_variant_only_folder_has_no_authoritative_manifest() {
        let temp = TempDir::new("variant-only");
        let folder = temp.addon("pfQuest");
        fs::write(folder.join("pfQuest-tbc.toc"), b"## Title: tbc\n").unwrap();
        assert!(find_toc(&folder).unwrap().is_none());
    }

    #[test]
    fn folder_without_any_manifest_is_not_an_error() {
        // The twelve Blizzard_* stub folders of a stock install look like this.
        let temp = TempDir::new("blizzard-stub");
        let folder = temp.addon("Blizzard_TalentUI");
        fs::write(folder.join("Blizzard_TalentUI.pub"), b"stub").unwrap();
        assert!(find_toc(&folder).unwrap().is_none());
        assert!(load(&folder).unwrap().is_none());
    }

    #[test]
    fn missing_folder_surfaces_the_io_error() {
        assert!(find_toc(Path::new("/definitely/not/here")).is_err());
        assert!(load(Path::new("/definitely/not/here")).is_err());
    }

    #[test]
    fn root_path_without_a_name_yields_nothing() {
        assert!(find_toc(Path::new("/")).unwrap().is_none());
    }

    #[test]
    fn load_reads_and_parses() {
        let temp = TempDir::new("load");
        let folder = temp.addon("Bagnon");
        fs::write(folder.join("Bagnon.toc"), SAMPLE).unwrap();
        let toc = load(&folder).unwrap().unwrap();
        assert_eq!(toc.display_title(), Some("pfQuest".to_string()));
        assert_eq!(toc.interface(), Some("11200"));
    }
}
