//! Shared utilities for the SingleStepTests/65x02 integration tests.

use serde::Deserialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
pub struct TestCase {
    pub name: String,
    pub initial: TestState,
    pub r#final: TestState,
    pub cycles: Vec<(u16, u8, String)>,
}

#[derive(Debug, Deserialize)]
pub struct TestState {
    pub pc: u16,
    pub s: u8,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub p: u8,
    pub ram: Vec<(u16, u8)>,
}

/// On first call, decompress the corpus tarball into
/// `target/test-cache/65x02/` and return that path. Subsequent calls
/// return the same path without re-extraction (idempotent via
/// directory-existence check).
pub fn corpus_root() -> &'static Path {
    static CACHE: OnceLock<PathBuf> = OnceLock::new();
    CACHE.get_or_init(|| {
        let target_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("target")
            .join("test-cache")
            .join("65x02");
        if !target_dir.exists() {
            extract_corpus(&target_dir);
        }
        target_dir
    })
}

fn extract_corpus(dest: &Path) {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/65x02.tar.zst");
    let f = std::fs::File::open(path).expect("open vendored corpus tarball");
    let dec = zstd::Decoder::new(f).expect("zstd decoder");
    let mut archive = tar::Archive::new(dec);
    std::fs::create_dir_all(dest).expect("create dest dir");
    archive.unpack(dest).expect("extract tarball");
}

/// Load all test cases for a specific opcode (e.g., 0xA9 for LDA #imm).
/// Returns the parsed test cases. Panics if the JSON file is missing.
pub fn load_opcode_cases(opcode: u8) -> Vec<TestCase> {
    let root = corpus_root();
    let filename = format!("{:02x}.json", opcode);
    // Search for the file under any subdirectory of root.
    let path = find_json(root, &filename)
        .unwrap_or_else(|| panic!("opcode {opcode:02X}: {filename} not found in corpus"));
    let mut buf = String::new();
    std::fs::File::open(&path)
        .unwrap_or_else(|e| panic!("open {path:?}: {e}"))
        .read_to_string(&mut buf)
        .expect("read corpus json");
    serde_json::from_str(&buf).unwrap_or_else(|e| panic!("parse {path:?}: {e}"))
}

fn find_json(root: &Path, filename: &str) -> Option<PathBuf> {
    fn recurse(dir: &Path, target: &str) -> Option<PathBuf> {
        let entries = std::fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() && p.file_name().is_some_and(|n| n == target) {
                return Some(p);
            }
            if p.is_dir() {
                if let Some(found) = recurse(&p, target) {
                    return Some(found);
                }
            }
        }
        None
    }
    recurse(root, filename)
}
