// Copyright (c) 2017 fd developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

extern crate kernel32;
extern crate winapi;

use kernel32::{GetStdHandle, GetConsoleMode, SetConsoleMode};
use winapi::{STD_OUTPUT_HANDLE, INVALID_HANDLE_VALUE};

const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;

// https://docs.microsoft.com/en-us/windows/console/console-virtual-terminal-sequences#example
pub fn enable_colored_output() -> bool {
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if handle == INVALID_HANDLE_VALUE {
            return false;
        }

        // https://docs.microsoft.com/en-us/windows/console/getconsolemode
        let mut mode = 0;
        if GetConsoleMode(handle, &mut mode) == 0 {
            return false;
        }
        mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING;

        // https://docs.microsoft.com/en-us/windows/console/setconsolemode
        //
        // A console consists of an input buffer and one or more screen buffers.  ...  Setting the
        // output modes of one screen buffer does not affect the output modes of other screen
        // buffers.
        SetConsoleMode(handle, mode) != 0
    }
}
