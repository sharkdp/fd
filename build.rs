#[macro_use]
extern crate clap;

use clap::Shell;

include!("src/app.rs");

fn main() {
    let outdir = match std::env::var_os("OUT_DIR") {
        None => return,
        Some(outdir) => outdir,
    };

    let mut app = build_app();
    app.gen_completions("fd", Shell::Bash, &outdir);
    app.gen_completions("fd", Shell::Fish, &outdir);
    app.gen_completions("fd", Shell::Zsh, &outdir);
    app.gen_completions("fd", Shell::PowerShell, &outdir);
}
