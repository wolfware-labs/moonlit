//! `moonlit version` — figlet-style banner + metadata. License is Elastic-2.0 (constraint
//! override of MVP_SPEC §9.2's stale `MIT`).

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
    println!("License: Elastic-2.0");
    0
}
