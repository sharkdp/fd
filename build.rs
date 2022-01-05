use std::fs;

use clap_complete::{generate_to, Shell};
use Shell::*;
//use clap_complete::shells::Shel{Bash, Fish, PowerShell, Elvish};

include!("src/app.rs");

fn main() {
    let min_version = "1.54";

    match version_check::is_min_version(min_version) {
        Some(true) => {}
        // rustc version too small or can't figure it out
        _ => {
            eprintln!("'fd' requires rustc >= {}", min_version);
            std::process::exit(1);
        }
    }

    let var = std::env::var_os("SHELL_COMPLETIONS_DIR").or_else(|| std::env::var_os("OUT_DIR"));
    let outdir = match var {
        None => return,
        Some(outdir) => outdir,
    };
    fs::create_dir_all(&outdir).unwrap();

    let mut app = build_app();
    // NOTE: zsh completions are hand written in contrib/completion/_fd
    for shell in [Bash, PowerShell, Fish, Elvish] {
        generate_to(shell, &mut app, "fd", &outdir).unwrap();
    }
}
