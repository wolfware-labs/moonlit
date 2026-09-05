//! The Rust implementation is the only Moonlit. Nothing in the source should
//! describe behaviour by reference to the retired C# engine.

use std::fs;
use std::path::Path;

fn rust_sources(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn sources_do_not_reference_the_csharp_engine() {
    let mut files = Vec::new();
    for root in ["src", "../cli/src"] {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(root);
        if p.is_dir() {
            rust_sources(&p, &mut files);
        }
    }
    assert!(!files.is_empty(), "found no sources to scan");

    let mut offenders = Vec::new();
    for f in &files {
        let text = fs::read_to_string(f).unwrap();
        for (i, line) in text.lines().enumerate() {
            let lower = line.to_lowercase();
            if line.contains("C#") || lower.contains("parity") || lower.contains("toclrtype") {
                offenders.push(format!("{}:{}: {}", f.display(), i + 1, line.trim()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "C#/parity references remain:\n{}",
        offenders.join("\n")
    );
}
