//! `cli/README.md` documents the top-level commands in a table. Nothing tied that table to the
//! actual command set, so `moonlit logout` shipped without ever being listed.
//!
//! Parse the command set out of `--help` and require a row for each.

use assert_cmd::Command;

/// Top-level subcommand names, read from the `Commands:` block of `moonlit --help`.
fn subcommands() -> Vec<String> {
    let out = Command::cargo_bin("moonlit")
        .unwrap()
        .arg("--help")
        .output()
        .unwrap();
    let help = String::from_utf8(out.stdout).unwrap();

    help.lines()
        .skip_while(|l| !l.starts_with("Commands:"))
        .skip(1)
        .take_while(|l| l.starts_with("  "))
        .filter_map(|l| l.split_whitespace().next())
        .filter(|name| *name != "help")
        .map(str::to_string)
        .collect()
}

#[test]
fn readme_documents_every_top_level_command() {
    let readme = include_str!("../README.md");
    let commands = subcommands();
    assert!(!commands.is_empty(), "parsed no commands out of --help");

    // A command counts as documented by its own row (`moonlit login`) or by rows for its
    // subcommands (`moonlit plugin new`, `moonlit plugin build`, …), so match the prefix.
    let missing: Vec<_> = commands
        .iter()
        .filter(|name| !readme.contains(&format!("`moonlit {name}")))
        .collect();

    assert!(
        missing.is_empty(),
        "cli/README.md's command table is missing: {missing:?}"
    );
}
