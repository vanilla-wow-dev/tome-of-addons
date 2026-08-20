//! `toa-tree-v1` — the canonical identity hash for a WoW addon folder.
//!
//! # Why not `git hash-object -t tree`?
//!
//! Git does not hash working-tree bytes; it hashes the *normalized* content in
//! its object store. `core.autocrlf` and `.gitattributes text=auto` are
//! checkout filters: content is normalized to LF on check-in and expanded
//! per-platform on check-out. Git is stable *because* it normalizes — not
//! because of the hash function.
//!
//! Tome of Addons hashes working trees directly, including ZIP-extracted
//! folders with no Git context at all. So the normalization has to happen here.
//! Without it the same addon would have a different identity depending on the
//! player's operating system, which would make the whole index worthless.
//!
//! # Normalization rules
//!
//! | Axis | Divergence between operating systems | Rule |
//! |---|---|---|
//! | Line endings | Windows check-out expands LF to CRLF | Text files: `CRLF` becomes `LF`. A lone `CR` is left alone — no OS filter ever produces one, so rewriting it would only corrupt content that legitimately contains it. |
//! | Exec bit | Windows has none (`100644` vs `100755`) | Mode is always `100644`. |
//! | Symlinks / submodules | Platform dependent, unreliable under Wine | Rejected, not silently skipped. |
//! | File name Unicode | macOS yields NFD, Linux/Windows NFC | Names are normalized to NFC. |
//! | Name collisions | `Foo.lua` + `foo.lua` coexist only on case-sensitive filesystems | Rejected — two names that normalize to the same key are an error, never a silent wrong hash. |
//! | OS artifacts | `.DS_Store`, `Thumbs.db`, `desktop.ini`, `._*` | Excluded. |
//! | Git metadata | `.git/` exists in developer mode, not in consumer mode | Excluded. |
//! | Empty directories | Git cannot represent them | Ignored. |
//!
//! Binary files are detected the way Git does it — a NUL byte within the first
//! 8000 bytes — and are hashed byte for byte, without line-ending rewriting.
//!
//! # Serialization
//!
//! Entries are serialized in Git's tree object format and sorted by Git's rule
//! (byte-wise over the name, with an implicit `/` appended for directories).
//! **SHA-256** runs over that serialization. The Git-shaped framing is kept
//! because it is well understood and verifiable — see the test suite, which
//! cross-checks the serialization against a real `git write-tree` by swapping
//! in SHA-1. Only the hash function and the pre-normalization differ from Git.

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use digest::Digest;
use sha2::Sha256;
use unicode_normalization::UnicodeNormalization;

/// Algorithm identifier. Belongs into every manifest and index entry so that a
/// future algorithm change can never silently compare old against new hashes.
pub const ALGO: &str = "toa-tree-v1";

/// Git blob mode. Always this one — the exec bit is normalized away.
const BLOB_MODE: &[u8] = b"100644";
/// Git tree mode (no leading zero, exactly as Git writes it).
const TREE_MODE: &[u8] = b"40000";
/// Number of leading bytes inspected for the binary heuristic (same as Git).
const BINARY_PROBE_BYTES: usize = 8000;
/// Files up to this size are read once into memory; larger ones are streamed in
/// two passes. Keeps peak memory bounded while giving the overwhelming majority
/// of addon files a single disk read.
const SLURP_LIMIT: u64 = 16 * 1024 * 1024;
/// Buffer size for the streaming path.
const CHUNK_SIZE: usize = 64 * 1024;

/// A canonical `toa-tree-v1` hash.
#[derive(Clone, PartialEq, Eq)]
pub struct TreeHash([u8; 32]);

impl TreeHash {
    /// Full 64-character lowercase hex representation.
    pub fn to_hex(&self) -> String {
        to_hex(&self.0)
    }

    /// First 12 hex characters — the form used in branch names and the UI.
    pub fn short(&self) -> String {
        self.to_hex()[..12].to_string()
    }
}

impl fmt::Display for TreeHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl fmt::Debug for TreeHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TreeHash({})", self.to_hex())
    }
}

/// Everything that can stop a tree from having a well-defined identity.
#[derive(Debug)]
pub enum HashError {
    /// Filesystem access failed.
    Io { path: PathBuf, source: io::Error },
    /// The hash root exists but is not a directory.
    NotADirectory(PathBuf),
    /// Symlinks have no OS-independent meaning and are never followed.
    Symlink(PathBuf),
    /// A file name that is not valid UTF-8 cannot be NFC-normalized.
    NonUtf8Name(PathBuf),
    /// Two entries in one directory normalize to the same key.
    NameCollision {
        dir: PathBuf,
        first: String,
        second: String,
    },
}

impl fmt::Display for HashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HashError::Io { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())
            }
            HashError::NotADirectory(path) => {
                write!(f, "not a directory: {}", path.display())
            }
            HashError::Symlink(path) => write!(
                f,
                "symlink is not hashable: {} — addon folders must be plain files",
                path.display()
            ),
            HashError::NonUtf8Name(path) => {
                write!(f, "file name is not valid UTF-8: {}", path.display())
            }
            HashError::NameCollision { dir, first, second } => write!(
                f,
                "{} contains {first:?} and {second:?}, which normalize to the same name — \
                 this folder cannot have a stable identity across operating systems",
                dir.display()
            ),
        }
    }
}

impl std::error::Error for HashError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HashError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Computes the canonical `toa-tree-v1` hash of an addon folder.
///
/// The folder's own name is deliberately *not* part of the hash: identity is
/// `(addon_id, tree_sha)`, and the id is tracked separately.
pub fn hash_tree(root: &Path) -> Result<TreeHash, HashError> {
    let raw = hash_dir::<Sha256>(root)?;
    let mut out = [0u8; 32];
    out.copy_from_slice(&raw);
    Ok(TreeHash(out))
}

/// One serialized tree entry.
struct Entry {
    name: String,
    is_tree: bool,
    hash: Vec<u8>,
}

/// Git's ordering rule: compare name bytes, but directories sort as if they
/// carried a trailing `/`. This is what makes `a.lua` sort before `a/`.
fn sort_key(entry: &Entry) -> Vec<u8> {
    let mut key = entry.name.as_bytes().to_vec();
    if entry.is_tree {
        key.push(b'/');
    }
    key
}

/// Hashes a directory as a tree object. Generic over the digest so the test
/// suite can substitute SHA-1 and compare against real `git write-tree`.
fn hash_dir<D: Digest>(dir: &Path) -> Result<Vec<u8>, HashError> {
    let meta = fs::symlink_metadata(dir).map_err(io_err(dir))?;
    if meta.file_type().is_symlink() {
        return Err(HashError::Symlink(dir.to_path_buf()));
    }
    if !meta.is_dir() {
        return Err(HashError::NotADirectory(dir.to_path_buf()));
    }
    let entries = collect_entries::<D>(dir)?;
    Ok(hash_object::<D>(b"tree", &serialize_tree(&entries)))
}

/// Walks one directory level, recursing into subdirectories. Returns entries
/// already sorted by Git's rule and verified free of name collisions.
fn collect_entries<D: Digest>(dir: &Path) -> Result<Vec<Entry>, HashError> {
    let mut entries = Vec::new();
    for item in fs::read_dir(dir).map_err(io_err(dir))? {
        let item = item.map_err(io_err(dir))?;
        let path = item.path();
        let raw_name = item.file_name();
        let name: String = raw_name
            .to_str()
            .ok_or_else(|| HashError::NonUtf8Name(path.clone()))?
            .nfc()
            .collect();
        if is_excluded(&name) {
            continue;
        }
        let file_type = item.file_type().map_err(io_err(&path))?;
        if file_type.is_symlink() {
            return Err(HashError::Symlink(path));
        }
        if file_type.is_dir() {
            let children = collect_entries::<D>(&path)?;
            // Git cannot represent empty directories, so neither do we. This
            // also covers directories that only held excluded artifacts.
            if children.is_empty() {
                continue;
            }
            entries.push(Entry {
                name,
                is_tree: true,
                hash: hash_object::<D>(b"tree", &serialize_tree(&children)),
            });
        } else if file_type.is_file() {
            entries.push(Entry {
                name,
                is_tree: false,
                hash: hash_blob::<D>(&path)?,
            });
        }
        // Anything else (fifo, socket, device node) cannot occur in an addon
        // folder and is skipped rather than turned into a hard error.
    }
    // Sorting before the collision check keeps the error message deterministic;
    // `read_dir` order is not.
    entries.sort_by_key(sort_key);
    check_collisions(dir, &entries)?;
    Ok(entries)
}

/// Rejects entries whose normalized names differ only in case. On Windows and
/// macOS only one of them would exist, so hashing both would silently produce a
/// platform-dependent identity.
fn check_collisions(dir: &Path, entries: &[Entry]) -> Result<(), HashError> {
    let mut seen: HashMap<String, String> = HashMap::new();
    for entry in entries {
        let key = entry.name.to_lowercase();
        if let Some(first) = seen.get(&key) {
            return Err(HashError::NameCollision {
                dir: dir.to_path_buf(),
                first: first.clone(),
                second: entry.name.clone(),
            });
        }
        seen.insert(key, entry.name.clone());
    }
    Ok(())
}

/// Names that never contribute to an addon's identity.
fn is_excluded(name: &str) -> bool {
    // AppleDouble sidecars appear whenever a macOS user touches a non-HFS volume.
    if name.starts_with("._") {
        return true;
    }
    matches!(
        name.to_ascii_lowercase().as_str(),
        ".git" | ".ds_store" | "thumbs.db" | "desktop.ini"
    )
}

/// Serializes sorted entries into Git's tree object body.
fn serialize_tree(entries: &[Entry]) -> Vec<u8> {
    let mut out = Vec::new();
    for entry in entries {
        out.extend_from_slice(if entry.is_tree { TREE_MODE } else { BLOB_MODE });
        out.push(b' ');
        out.extend_from_slice(entry.name.as_bytes());
        out.push(0);
        out.extend_from_slice(&entry.hash);
    }
    out
}

/// Wraps content in Git's `<kind> <len>\0<content>` framing and digests it.
fn hash_object<D: Digest>(kind: &[u8], content: &[u8]) -> Vec<u8> {
    let mut digest = D::new();
    digest.update(kind);
    digest.update(b" ");
    digest.update(content.len().to_string().as_bytes());
    digest.update([0u8]);
    digest.update(content);
    digest.finalize().to_vec()
}

fn hash_blob<D: Digest>(path: &Path) -> Result<Vec<u8>, HashError> {
    hash_blob_tuned::<D>(path, SLURP_LIMIT, CHUNK_SIZE)
}

/// Blob hashing with explicit tuning knobs so the streaming path can be
/// exercised in tests without writing 16 MB fixtures.
fn hash_blob_tuned<D: Digest>(
    path: &Path,
    slurp_limit: u64,
    chunk: usize,
) -> Result<Vec<u8>, HashError> {
    let mut file = File::open(path).map_err(io_err(path))?;
    let size = file.metadata().map_err(io_err(path))?.len();

    if size <= slurp_limit {
        let mut buf = Vec::with_capacity(size as usize);
        file.read_to_end(&mut buf).map_err(io_err(path))?;
        let content: Cow<'_, [u8]> = if is_binary(&buf) {
            Cow::Borrowed(&buf)
        } else {
            normalize_crlf(&buf)
        };
        return Ok(hash_object::<D>(b"blob", &content));
    }

    // Large file: two streaming passes. The object header needs the *normalized*
    // length, which is only known after a full scan. The second pass reads from
    // the page cache the first one just warmed, so the extra cost is memory
    // bandwidth rather than disk I/O.
    let binary = probe_binary(&mut file).map_err(io_err(path))?;

    let mut length = 0usize;
    file.seek(SeekFrom::Start(0)).map_err(io_err(path))?;
    stream_normalized(&mut file, binary, chunk, |bytes| length += bytes.len())
        .map_err(io_err(path))?;

    let mut digest = D::new();
    digest.update(b"blob ");
    digest.update(length.to_string().as_bytes());
    digest.update([0u8]);
    file.seek(SeekFrom::Start(0)).map_err(io_err(path))?;
    stream_normalized(&mut file, binary, chunk, |bytes| digest.update(bytes))
        .map_err(io_err(path))?;
    Ok(digest.finalize().to_vec())
}

/// Reads the leading probe window and applies the binary heuristic.
fn probe_binary(file: &mut File) -> io::Result<bool> {
    let mut buf = vec![0u8; BINARY_PROBE_BYTES];
    let mut filled = 0;
    while filled < buf.len() {
        let read = file.read(&mut buf[filled..])?;
        if read == 0 {
            break;
        }
        filled += read;
    }
    Ok(is_binary(&buf[..filled]))
}

/// Git's heuristic: a NUL byte in the probe window means binary.
fn is_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(BINARY_PROBE_BYTES).any(|&b| b == 0)
}

/// Offset of the next `CRLF` pair, skipping lone `CR` bytes.
fn find_crlf(bytes: &[u8]) -> Option<usize> {
    let mut from = 0;
    while let Some(offset) = bytes[from..].iter().position(|&b| b == b'\r') {
        let pos = from + offset;
        if bytes.get(pos + 1) == Some(&b'\n') {
            return Some(pos);
        }
        from = pos + 1;
    }
    None
}

/// Rewrites `CRLF` to `LF`, borrowing when there is nothing to do.
///
/// Copies run-wise rather than byte-wise: on a 435 MB addon corpus a per-byte
/// push loop cost several seconds, which dominated the SHA-256 work itself.
/// The search doubles as the "is there anything to do at all" check, so text
/// files are scanned once, not twice.
fn normalize_crlf(bytes: &[u8]) -> Cow<'_, [u8]> {
    let Some(first) = find_crlf(bytes) else {
        return Cow::Borrowed(bytes);
    };
    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(&bytes[..first]);
    let mut i = first;
    while i < bytes.len() {
        // Invariant: `i` points at the `\r` of a known CRLF pair.
        out.push(b'\n');
        i += 2;
        let rest = &bytes[i..];
        match find_crlf(rest) {
            Some(next) => {
                out.extend_from_slice(&rest[..next]);
                i += next;
            }
            None => {
                out.extend_from_slice(rest);
                i = bytes.len();
            }
        }
    }
    Cow::Owned(out)
}

/// Feeds normalized bytes to `sink` in chunks. A `CR` landing on a chunk
/// boundary is held back until the next chunk reveals whether it starts a CRLF.
fn stream_normalized<R: Read, F: FnMut(&[u8])>(
    reader: &mut R,
    binary: bool,
    chunk: usize,
    mut sink: F,
) -> io::Result<()> {
    let mut buf = vec![0u8; chunk];
    let mut pending_cr = false;
    loop {
        let read = reader.read(&mut buf)?;
        if read == 0 {
            break;
        }
        let data = &buf[..read];
        if binary {
            sink(data);
            continue;
        }
        let mut i = 0;
        if pending_cr {
            pending_cr = false;
            if data[0] == b'\n' {
                sink(b"\n");
                i = 1;
            } else {
                sink(b"\r");
            }
        }
        while i < data.len() {
            match data[i..].iter().position(|&b| b == b'\r') {
                None => {
                    sink(&data[i..]);
                    i = data.len();
                }
                Some(offset) => {
                    let pos = i + offset;
                    if pos > i {
                        sink(&data[i..pos]);
                    }
                    match data.get(pos + 1) {
                        Some(b'\n') => {
                            sink(b"\n");
                            i = pos + 2;
                        }
                        Some(_) => {
                            sink(b"\r");
                            i = pos + 1;
                        }
                        None => {
                            pending_cr = true;
                            i = pos + 1;
                        }
                    }
                }
            }
        }
    }
    if pending_cr {
        sink(b"\r");
    }
    Ok(())
}

fn io_err(path: &Path) -> impl FnOnce(io::Error) -> HashError + '_ {
    move |source| HashError::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from_digit((byte >> 4) as u32, 16).unwrap());
        out.push(char::from_digit((byte & 0x0f) as u32, 16).unwrap());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// Throwaway directory tree. Unique without needing a clock or RNG.
    struct TempTree {
        root: PathBuf,
    }

    impl TempTree {
        fn new(label: &str) -> Self {
            let id = COUNTER.fetch_add(1, Ordering::SeqCst);
            let root = std::env::temp_dir().join(format!(
                "toa-tree-{}-{}-{}",
                std::process::id(),
                id,
                label
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            TempTree { root }
        }

        fn file(&self, rel: &str, content: &[u8]) -> &Self {
            let path = self.root.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, content).unwrap();
            self
        }

        fn dir(&self, rel: &str) -> &Self {
            fs::create_dir_all(self.root.join(rel)).unwrap();
            self
        }

        fn path(&self) -> &Path {
            &self.root
        }

        fn hash(&self) -> TreeHash {
            hash_tree(&self.root).expect("tree should hash")
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    // ---------------------------------------------------------------- basics

    #[test]
    fn same_content_hashes_the_same() {
        let a = TempTree::new("same-a");
        a.file("core.lua", b"print('hi')\n")
            .file("pfUI.toc", b"## Title: pfUI\n");
        let b = TempTree::new("same-b");
        b.file("core.lua", b"print('hi')\n")
            .file("pfUI.toc", b"## Title: pfUI\n");
        assert_eq!(a.hash(), b.hash());
    }

    #[test]
    fn different_content_hashes_differently() {
        let a = TempTree::new("diff-a");
        a.file("core.lua", b"print('hi')\n");
        let b = TempTree::new("diff-b");
        b.file("core.lua", b"print('ho')\n");
        assert_ne!(a.hash(), b.hash());
    }

    #[test]
    fn folder_name_is_not_part_of_the_identity() {
        let a = TempTree::new("name-pfui");
        a.file("core.lua", b"x\n");
        let b = TempTree::new("name-bagnon");
        b.file("core.lua", b"x\n");
        assert_eq!(a.hash(), b.hash());
    }

    #[test]
    fn empty_tree_is_hashable() {
        let empty = TempTree::new("empty");
        // Same framing Git uses for the empty tree, so the value is predictable.
        assert_eq!(empty.hash(), TreeHash(Sha256::digest(b"tree 0\0").into()));
    }

    // --------------------------------------------------- normalization axes

    #[test]
    fn crlf_and_lf_hash_identically() {
        let lf = TempTree::new("lf");
        lf.file("core.lua", b"local a = 1\nlocal b = 2\n");
        let crlf = TempTree::new("crlf");
        crlf.file("core.lua", b"local a = 1\r\nlocal b = 2\r\n");
        assert_eq!(lf.hash(), crlf.hash());
    }

    #[test]
    fn lone_cr_is_preserved_and_not_rewritten() {
        // No OS check-out filter produces a lone CR, so it is real content.
        let cr = TempTree::new("lone-cr");
        cr.file("core.lua", b"a\rb\n");
        let lf = TempTree::new("lone-cr-as-lf");
        lf.file("core.lua", b"a\nb\n");
        assert_ne!(cr.hash(), lf.hash());
    }

    #[test]
    fn binary_files_are_not_line_ending_normalized() {
        let with_crlf = TempTree::new("bin-crlf");
        with_crlf.file("logo.tga", b"\0\x01\x02\r\n\x03");
        let with_lf = TempTree::new("bin-lf");
        with_lf.file("logo.tga", b"\0\x01\x02\n\x03");
        assert_ne!(with_crlf.hash(), with_lf.hash());
    }

    #[test]
    fn nul_after_the_probe_window_still_counts_as_text() {
        let mut text = vec![b'a'; BINARY_PROBE_BYTES + 100];
        text[BINARY_PROBE_BYTES + 50] = 0;
        assert!(!is_binary(&text));
    }

    #[test]
    #[cfg(unix)]
    fn exec_bit_does_not_change_the_hash() {
        use std::os::unix::fs::PermissionsExt;
        let plain = TempTree::new("mode-plain");
        plain.file("core.lua", b"x\n");
        let exec = TempTree::new("mode-exec");
        exec.file("core.lua", b"x\n");
        fs::set_permissions(
            exec.path().join("core.lua"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        assert_eq!(plain.hash(), exec.hash());
    }

    #[test]
    fn nfd_and_nfc_file_names_hash_identically() {
        // macOS hands out NFD ("e" + combining acute), Linux/Windows NFC ("é").
        let nfd = TempTree::new("nfd");
        nfd.file("Cafe\u{0301}.lua", b"x\n");
        let nfc = TempTree::new("nfc");
        nfc.file("Caf\u{00e9}.lua", b"x\n");
        assert_eq!(nfd.hash(), nfc.hash());
    }

    // ------------------------------------------------------------ exclusions

    #[test]
    fn os_artifacts_are_ignored() {
        let clean = TempTree::new("artifacts-clean");
        clean.file("core.lua", b"x\n");
        let dirty = TempTree::new("artifacts-dirty");
        dirty
            .file("core.lua", b"x\n")
            .file(".DS_Store", b"junk")
            .file("Thumbs.db", b"junk")
            .file("desktop.ini", b"junk")
            .file("._core.lua", b"junk");
        assert_eq!(clean.hash(), dirty.hash());
    }

    #[test]
    fn git_directory_is_ignored() {
        let consumer = TempTree::new("consumer");
        consumer.file("core.lua", b"x\n");
        let developer = TempTree::new("developer");
        developer
            .file("core.lua", b"x\n")
            .file(".git/HEAD", b"ref: refs/heads/master\n")
            .file(".git/config", b"[core]\n");
        assert_eq!(consumer.hash(), developer.hash());
    }

    #[test]
    fn empty_directories_are_ignored() {
        let flat = TempTree::new("dirs-flat");
        flat.file("core.lua", b"x\n");
        let with_empty = TempTree::new("dirs-empty");
        with_empty.file("core.lua", b"x\n").dir("img").dir("a/b/c");
        assert_eq!(flat.hash(), with_empty.hash());
    }

    #[test]
    fn directory_holding_only_artifacts_is_ignored() {
        let flat = TempTree::new("only-artifacts-flat");
        flat.file("core.lua", b"x\n");
        let nested = TempTree::new("only-artifacts-nested");
        nested
            .file("core.lua", b"x\n")
            .file("img/.DS_Store", b"junk");
        assert_eq!(flat.hash(), nested.hash());
    }

    #[test]
    #[cfg(unix)]
    fn non_regular_files_are_skipped() {
        // Neither a file nor a directory: a FIFO carries no content that could
        // belong to an addon's identity, so it must not affect the hash.
        let plain = TempTree::new("fifo-plain");
        plain.file("core.lua", b"x\n");
        let with_fifo = TempTree::new("fifo-present");
        with_fifo.file("core.lua", b"x\n");
        let status = std::process::Command::new("mkfifo")
            .arg(with_fifo.path().join("pipe"))
            .status()
            .expect("mkfifo must be on PATH");
        assert!(status.success());
        assert_eq!(plain.hash(), with_fifo.hash());
    }

    #[test]
    fn nested_content_still_counts() {
        let a = TempTree::new("nested-a");
        a.file("img/icon.tga", b"\0\x01");
        let b = TempTree::new("nested-b");
        b.file("img/icon.tga", b"\0\x02");
        assert_ne!(a.hash(), b.hash());
    }

    // ---------------------------------------------------------------- errors

    #[test]
    fn missing_root_is_an_io_error() {
        let err = hash_tree(Path::new("/definitely/not/here")).unwrap_err();
        assert!(matches!(err, HashError::Io { .. }));
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn file_as_root_is_rejected() {
        let tree = TempTree::new("root-is-file");
        tree.file("core.lua", b"x\n");
        let err = hash_tree(&tree.path().join("core.lua")).unwrap_err();
        assert!(matches!(err, HashError::NotADirectory(_)));
        assert!(std::error::Error::source(&err).is_none());
    }

    #[test]
    #[cfg(unix)]
    fn symlink_inside_the_tree_is_rejected() {
        let tree = TempTree::new("symlink-inner");
        tree.file("core.lua", b"x\n");
        std::os::unix::fs::symlink("core.lua", tree.path().join("link.lua")).unwrap();
        assert!(matches!(
            hash_tree(tree.path()).unwrap_err(),
            HashError::Symlink(_)
        ));
    }

    #[test]
    #[cfg(unix)]
    fn symlinked_root_is_rejected() {
        let tree = TempTree::new("symlink-root");
        tree.file("core.lua", b"x\n");
        let link = tree.path().with_extension("link");
        let _ = fs::remove_file(&link);
        std::os::unix::fs::symlink(tree.path(), &link).unwrap();
        let result = hash_tree(&link);
        let _ = fs::remove_file(&link);
        assert!(matches!(result.unwrap_err(), HashError::Symlink(_)));
    }

    #[test]
    #[cfg(unix)]
    fn case_only_name_collision_is_rejected() {
        let tree = TempTree::new("case-collision");
        tree.file("Core.lua", b"x\n").file("core.lua", b"y\n");
        let err = hash_tree(tree.path()).unwrap_err();
        assert!(matches!(err, HashError::NameCollision { .. }));
        // Deterministic ordering: entries are sorted before the check.
        assert!(err.to_string().contains("\"Core.lua\""));
    }

    #[test]
    #[cfg(unix)]
    fn unicode_normalization_collision_is_rejected() {
        // Two distinct files on Linux, one single file on macOS.
        let tree = TempTree::new("nfc-collision");
        tree.file("Cafe\u{0301}.lua", b"x\n")
            .file("Caf\u{00e9}.lua", b"y\n");
        assert!(matches!(
            hash_tree(tree.path()).unwrap_err(),
            HashError::NameCollision { .. }
        ));
    }

    #[test]
    #[cfg(unix)]
    fn non_utf8_file_name_is_rejected() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let tree = TempTree::new("non-utf8");
        let name = OsStr::from_bytes(b"broken-\xff.lua");
        fs::write(tree.path().join(name), b"x").unwrap();
        assert!(matches!(
            hash_tree(tree.path()).unwrap_err(),
            HashError::NonUtf8Name(_)
        ));
    }

    #[test]
    fn every_error_variant_renders() {
        let variants = [
            HashError::Io {
                path: "/p".into(),
                source: io::Error::other("boom"),
            },
            HashError::NotADirectory("/p".into()),
            HashError::Symlink("/p".into()),
            HashError::NonUtf8Name("/p".into()),
            HashError::NameCollision {
                dir: "/p".into(),
                first: "A".into(),
                second: "a".into(),
            },
        ];
        for variant in &variants {
            assert!(!variant.to_string().is_empty());
            assert!(!format!("{variant:?}").is_empty());
        }
    }

    // ------------------------------------------------------- streaming path

    #[test]
    fn streaming_matches_slurping() {
        let tree = TempTree::new("stream-equiv");
        // Deliberately mixes CRLF, lone CR and a trailing CR.
        let mut content = Vec::new();
        for i in 0..5000 {
            content.extend_from_slice(format!("line {i}\r\n").as_bytes());
            content.extend_from_slice(b"bare\rcr\n");
        }
        content.push(b'\r');
        tree.file("big.lua", &content);
        let path = tree.path().join("big.lua");

        let slurped = hash_blob_tuned::<Sha256>(&path, u64::MAX, CHUNK_SIZE).unwrap();
        for chunk in [1usize, 2, 3, 7, 64, 4096] {
            let streamed = hash_blob_tuned::<Sha256>(&path, 0, chunk).unwrap();
            assert_eq!(slurped, streamed, "chunk size {chunk} diverged");
        }
    }

    #[test]
    fn cr_exactly_on_a_chunk_boundary() {
        let tree = TempTree::new("cr-boundary");
        // With chunk size 4 the CR is the last byte of the first chunk.
        tree.file("a.lua", b"abc\r\ndef\n");
        let path = tree.path().join("a.lua");
        let reference = hash_blob_tuned::<Sha256>(&path, u64::MAX, CHUNK_SIZE).unwrap();
        assert_eq!(reference, hash_blob_tuned::<Sha256>(&path, 0, 4).unwrap());

        // Trailing CR at the very end of a chunk, with nothing following.
        tree.file("b.lua", b"abc\r");
        let path = tree.path().join("b.lua");
        let reference = hash_blob_tuned::<Sha256>(&path, u64::MAX, CHUNK_SIZE).unwrap();
        assert_eq!(reference, hash_blob_tuned::<Sha256>(&path, 0, 4).unwrap());
    }

    #[test]
    fn streaming_binary_is_passed_through_untouched() {
        let tree = TempTree::new("stream-binary");
        let mut content = vec![0u8, 1, 2];
        content.extend(std::iter::repeat_n(b'\r', 10));
        content.extend_from_slice(b"\r\n");
        tree.file("blob.bin", &content);
        let path = tree.path().join("blob.bin");
        assert_eq!(
            hash_blob_tuned::<Sha256>(&path, u64::MAX, CHUNK_SIZE).unwrap(),
            hash_blob_tuned::<Sha256>(&path, 0, 4).unwrap()
        );
    }

    #[test]
    fn probe_reads_the_full_window_across_short_reads() {
        let tree = TempTree::new("probe");
        let mut content = vec![b'a'; 32];
        content[10] = 0;
        tree.file("x.bin", &content);
        let mut file = File::open(tree.path().join("x.bin")).unwrap();
        assert!(probe_binary(&mut file).unwrap());
    }

    // -------------------------------------------------------- helper units

    #[test]
    fn normalize_borrows_when_there_is_nothing_to_do() {
        assert!(matches!(normalize_crlf(b"plain\n"), Cow::Borrowed(_)));
        // Lone CRs alone are not a reason to copy.
        assert!(matches!(normalize_crlf(b"a\rb\rc"), Cow::Borrowed(_)));
        assert!(matches!(normalize_crlf(b"a\r\nb"), Cow::Owned(_)));
    }

    #[test]
    fn normalize_rewrites_only_crlf_pairs() {
        assert_eq!(normalize_crlf(b"a\r\nb").as_ref(), b"a\nb");
        // Leading pair, trailing pair, and back-to-back pairs.
        assert_eq!(normalize_crlf(b"\r\na\r\n\r\n").as_ref(), b"\na\n\n");
        // Lone CRs survive, including one directly before a rewritten pair and
        // one at the very end.
        assert_eq!(normalize_crlf(b"a\r\r\nb\r").as_ref(), b"a\r\nb\r");
        assert_eq!(normalize_crlf(b"a\r\nb\r").as_ref(), b"a\nb\r");
    }

    #[test]
    fn find_crlf_skips_lone_carriage_returns() {
        assert_eq!(find_crlf(b"abc"), None);
        assert_eq!(find_crlf(b"a\rb\rc"), None);
        assert_eq!(find_crlf(b"a\r"), None);
        assert_eq!(find_crlf(b"a\r\n"), Some(1));
        assert_eq!(find_crlf(b"a\r\rb\r\n"), Some(4));
    }

    #[test]
    fn exclusion_list_is_case_insensitive() {
        assert!(is_excluded(".git"));
        assert!(is_excluded(".DS_Store"));
        assert!(is_excluded("thumbs.db"));
        assert!(is_excluded("Desktop.ini"));
        assert!(is_excluded("._resource"));
        assert!(!is_excluded(".gitattributes"));
        assert!(!is_excluded("core.lua"));
    }

    #[test]
    fn hex_covers_the_whole_byte_range() {
        assert_eq!(to_hex(&[0x00, 0x0f, 0xa5, 0xff]), "000fa5ff");
    }

    #[test]
    fn tree_hash_renders_short_and_long() {
        let tree = TempTree::new("render");
        tree.file("core.lua", b"x\n");
        let hash = tree.hash();
        assert_eq!(hash.to_hex().len(), 64);
        assert_eq!(hash.short().len(), 12);
        assert!(hash.to_hex().starts_with(&hash.short()));
        assert_eq!(hash.to_string(), hash.to_hex());
        assert_eq!(format!("{hash:?}"), format!("TreeHash({})", hash.to_hex()));
        assert_eq!(hash.clone(), hash);
    }

    #[test]
    fn directories_sort_as_if_they_ended_in_a_slash() {
        let file = Entry {
            name: "a.lua".into(),
            is_tree: false,
            hash: vec![],
        };
        let dir = Entry {
            name: "a".into(),
            is_tree: true,
            hash: vec![],
        };
        // '.' (0x2e) < '/' (0x2f), so the file sorts first.
        assert!(sort_key(&file) < sort_key(&dir));
    }

    // ------------------------------------------------- git cross-validation

    /// Runs `git write-tree` over `dir` and returns the tree SHA-1.
    fn git_write_tree(dir: &Path) -> String {
        let run = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .args(args)
                .current_dir(dir)
                .output()
                .expect("git must be on PATH for the cross-check");
            // Rendered unconditionally so the success path covers this line too.
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert!(out.status.success(), "git {args:?} failed: {stderr}");
            String::from_utf8(out.stdout).unwrap()
        };
        run(&["init", "-q"]);
        // Pin everything that could otherwise be inherited from the developer's
        // global config and quietly rewrite content.
        run(&["config", "core.autocrlf", "false"]);
        run(&["config", "core.eol", "lf"]);
        run(&["config", "core.excludesFile", ""]);
        run(&["add", "-A", "-f", "--", "."]);
        run(&["write-tree"]).trim().to_string()
    }

    /// The serialization and sorting rules are validated against real Git by
    /// swapping SHA-256 for SHA-1. If this passes, the only intentional
    /// deviations from Git are the hash function and the pre-normalization.
    #[test]
    fn serialization_matches_git_write_tree() {
        let tree = TempTree::new("git-compat");
        tree
            // Names chosen to exercise the trailing-slash sorting rule:
            // "a.lua" vs the directory "a" vs "a-b.lua" vs "a0.lua".
            .file("a.lua", b"one\n")
            .file("a-b.lua", b"two\n")
            .file("a0.lua", b"three\n")
            .file("a/nested.lua", b"four\n")
            .file("a/deep/deeper.lua", b"five\n")
            .file("Zed.toc", b"## Title: Zed\n")
            .file("img/icon.tga", b"\0\x01\x02\x03")
            .file("UPPER.lua", b"six\n")
            .file("_under.lua", b"seven\n");

        let ours = to_hex(&hash_dir::<sha1::Sha1>(tree.path()).unwrap());
        assert_eq!(ours, git_write_tree(tree.path()));
    }

    #[test]
    fn serialization_matches_git_for_an_empty_tree() {
        let tree = TempTree::new("git-compat-empty");
        let ours = to_hex(&hash_dir::<sha1::Sha1>(tree.path()).unwrap());
        // Git's well-known empty tree object.
        assert_eq!(ours, "4b825dc642cb6eb9a060e54bf8d69288fbee4904");
        assert_eq!(ours, git_write_tree(tree.path()));
    }
}
