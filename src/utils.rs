// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use std::path::Path;
use std::iter::Iterator;

/// Determine if an os string ends with any of the given extensions (case insensitive).
pub fn path_has_any_extension<'a, I>(path: &Path, exts: I) -> bool
where
    I: 'a + Iterator<Item = &'a String> + Clone,
{
    // TODO: remove these two lines when we drop support for Rust version < 1.23.
    #[allow(unused_imports)]
    use std::ascii::AsciiExt;

    if let Some(ref name) = path.file_name() {
        if let Some(ref name_str) = name.to_str() {
            exts.clone().any(|x| {
                let mut it = name_str.chars().rev();

                if x.chars()
                    .rev()
                    .zip(&mut it)
                    .all(|(a, b)| a.eq_ignore_ascii_case(&b))
                {
                    match it.next() {
                        Some('/') | None => false,
                        _ => true,
                    }
                } else {
                    false
                }
            })
        } else {
            false
        }
    } else {
        false
    }
}
