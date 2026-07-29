//! `moonlit version` — figlet-style banner plus name, version, author, and license.

const BANNER: &str = r"
 __  __                _ _ _
|  \/  | ___   ___  _ __ | (_) |_
| |\/| |/ _ \ / _ \| '_ \| | | __|
| |  | | (_) | (_) | | | | | | |_
|_|  |_|\___/ \___/|_| |_|_|_|\__|";

pub fn run() -> i32 {
    println!("{BANNER}");
    println!("Moonlit v{}", env!("CARGO_PKG_VERSION"));
    println!("Author: Wolfware LLC");
    println!("License: MIT OR Apache-2.0");
    0
}
