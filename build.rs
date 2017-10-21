// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

#[macro_use]
extern crate clap;

use clap::Shell;

include!("src/app.rs");

fn main() {
    let var = std::env::var_os("SHELL_COMPLETIONS_DIR").or(std::env::var_os("OUT_DIR"));
    let outdir = match var {
        None => return,
        Some(outdir) => outdir,
    };

    let mut app = build_app();
    app.gen_completions("fd", Shell::Bash, &outdir);
    app.gen_completions("fd", Shell::Fish, &outdir);
    app.gen_completions("fd", Shell::Zsh, &outdir);
    app.gen_completions("fd", Shell::PowerShell, &outdir);
}
