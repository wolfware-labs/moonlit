//! `moonlit version` — the wolf-and-moon logo beside the name, slogan, version,
//! author, and license.
//!
//! On a wide, truecolor terminal it renders the logo as color half-blocks with
//! the text set beside it. When color is unavailable (piped output, `NO_COLOR`,
//! a narrow or non-truecolor terminal) it falls back to a plain figlet banner so
//! logs and pipes stay clean.

use console::Term;

const BANNER: &str = r"
 __  __                _ _ _
|  \/  | ___   ___  _ __ | (_) |_
| |\/| |/ _ \ / _ \| '_ \| | | __|
| |  | | (_) | (_) | | | | | | |_
|_|  |_|\___/ \___/|_| |_|_|_|\__|";

/// Truecolor half-block rendering of the Moonlit wolf-and-moon logo,
/// generated from `docs/public/logo.png` with `chafa`.
const LOGO: &str = include_str!("moonlit_logo.ansi");

const SLOGAN: &str = "Bring light to your release process";
const AUTHOR: &str = "Wolfware LLC";
const LICENSE: &str = "MIT OR Apache-2.0";
// Show the full https URL so terminals that auto-linkify bare URLs make it
// clickable even without OSC 8 support.
const HOMEPAGE: &str = "https://moonlitbuild.dev";
const HOMEPAGE_URL: &str = "https://moonlitbuild.dev/";

// Moon palette (truecolor; only used on the truecolor-gated fancy path).
// Each text element gets its own hue.
const SKY: &str = "\x1b[38;2;143;199;232m"; // moonlight blue — the wordmark
const MOON: &str = "\x1b[38;2;238;225;193m"; // moon cream — the version
const LILAC: &str = "\x1b[38;2;185;167;230m"; // periwinkle — the slogan
const SLATE: &str = "\x1b[38;2;134;141;151m"; // slate grey — author / license
const TEAL: &str = "\x1b[38;2;111;211;184m"; // mint — the docs link
const BOLD: &str = "\x1b[1m";
const RST: &str = "\x1b[0m";

/// Column (1-based) where the text block begins, just past the ~34-wide logo.
const TEXT_COL: usize = 39;

pub fn run() -> i32 {
    let version = env!("CARGO_PKG_VERSION");
    if fancy() {
        print_fancy(version);
    } else {
        print_plain(version);
    }
    0
}

/// True when we can render the colored side-by-side layout: a truecolor,
/// color-enabled terminal wide enough for the logo plus the text.
fn fancy() -> bool {
    let truecolor = std::env::var("COLORTERM")
        .map(|v| v.contains("truecolor") || v.contains("24bit"))
        .unwrap_or(false);
    let wide_enough = Term::stdout().size().1 as usize >= 76;
    console::colors_enabled() && truecolor && wide_enough
}

fn print_plain(version: &str) {
    println!("{BANNER}");
    println!("Moonlit v{version}");
    println!("{SLOGAN}");
    println!("Author: {AUTHOR}");
    println!("License: {LICENSE}");
}

fn print_fancy(version: &str) {
    // Text rows placed beside the logo (vertically centered against it).
    let rows = [
        format!("{BOLD}{SKY}Moonlit{RST}  {MOON}v{version}{RST}"),
        format!("{LILAC}{SLOGAN}{RST}"),
        String::new(),
        format!("{SLATE}{AUTHOR}  ·  {LICENSE}{RST}"),
        // OSC 8 hyperlink (ST-terminated) over a full-URL label: clickable via
        // the hyperlink where supported, and via bare-URL auto-detection otherwise.
        // No explicit underline — terminals underline links on hover themselves.
        format!("{TEAL}→ \x1b]8;;{HOMEPAGE_URL}\x1b\\{HOMEPAGE}\x1b]8;;\x1b\\{RST}"),
    ];

    let logo: Vec<&str> = LOGO.lines().collect();
    let start = logo.len().saturating_sub(rows.len()) / 2;

    for (i, line) in logo.iter().enumerate() {
        print!("{line}");
        if let Some(text) = i.checked_sub(start).and_then(|r| rows.get(r)) {
            // Reset any lingering logo color, jump to the text column, print.
            print!("{RST}\x1b[{TEXT_COL}G{text}");
        }
        println!();
    }
}
