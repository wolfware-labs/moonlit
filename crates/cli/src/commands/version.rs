use console::Term;

const BANNER: &str = r"
 __  __                   _ _ _
|  \/  | ___   ___  _ __ | (_) |_
| |\/| |/ _ \ / _ \| '_ \| | | __|
| |  | | (_) | (_) | | | | | | |_
|_|  |_|\___/ \___/|_| |_|_|_|\__|";

const LOGO: &str = include_str!("moonlit_logo.ansi");
const SLOGAN: &str = "Bring light to your release process";
const AUTHOR: &str = "Wolfware LLC";
const LICENSE: &str = "MIT OR Apache-2.0";
const HOMEPAGE: &str = "https://moonlit.rs";
const HOMEPAGE_URL: &str = "https://moonlit.rs/";

const SKY: &str = "\x1b[38;2;143;199;232m";
const MOON: &str = "\x1b[38;2;238;225;193m";
const LILAC: &str = "\x1b[38;2;185;167;230m";
const SLATE: &str = "\x1b[38;2;134;141;151m";
const TEAL: &str = "\x1b[38;2;111;211;184m";
const BOLD: &str = "\x1b[1m";
const RST: &str = "\x1b[0m";

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
    let rows = [
        format!("{BOLD}{SKY}Moonlit{RST}  {MOON}v{version}{RST}"),
        format!("{LILAC}{SLOGAN}{RST}"),
        String::new(),
        format!("{SLATE}{AUTHOR}  ·  {LICENSE}{RST}"),
        format!("{TEAL}→ \x1b]8;;{HOMEPAGE_URL}\x1b\\{HOMEPAGE}\x1b]8;;\x1b\\{RST}"),
    ];

    let logo: Vec<&str> = LOGO.lines().collect();
    let start = logo.len().saturating_sub(rows.len()) / 2;

    for (i, line) in logo.iter().enumerate() {
        print!("{line}");
        if let Some(text) = i.checked_sub(start).and_then(|r| rows.get(r)) {
            print!("{RST}\x1b[{TEXT_COL}G{text}");
        }
        println!();
    }
}
