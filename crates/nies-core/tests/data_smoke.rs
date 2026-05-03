//! Smoke test: confirm the vendored 65x02 corpus tarball decompresses
//! and contains valid JSON for at least one opcode (LDA #imm, $A9).

use std::io::Read;

#[test]
fn corpus_tarball_contains_a9_lda_imm() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/65x02.tar.zst");
    let f = std::fs::File::open(path).expect("open tarball");
    let dec = zstd::Decoder::new(f).expect("zstd decoder");
    let mut archive = tar::Archive::new(dec);

    for entry in archive.entries().expect("read entries") {
        let mut entry = entry.expect("entry");
        let path = entry.path().expect("path").to_path_buf();
        if path.file_name().is_some_and(|n| n == "a9.json") {
            let mut buf = String::new();
            entry.read_to_string(&mut buf).expect("read a9.json");
            // Parse as untyped JSON; just confirm it's an array with > 0 cases.
            let parsed: serde_json::Value =
                serde_json::from_str(&buf).expect("parse a9.json");
            let cases = parsed.as_array().expect("array");
            assert!(cases.len() > 100, "expected lots of test cases, got {}", cases.len());
            return;
        }
    }
    panic!("a9.json not found in corpus tarball");
}
