use std::borrow::Cow;

#[cfg(windows)]
const ESCAPE_CHAR: u8 = b'`';

#[cfg(not(windows))]
const ESCAPE_CHAR: u8 = b'\\';

fn needs_escape(byte: u8) -> bool {
    byte < 43 || (byte > 58 && byte < 65) || (byte > 90 && byte < 97) || (byte > 122 && byte <= 127)
}

pub fn escape<'a>(input: &'a str) -> Cow<'a, str> {
    let chars_to_escape = input
        .as_bytes()
        .iter()
        .filter(|&&x| needs_escape(x))
        .count();
    if chars_to_escape == 0 {
        Cow::Borrowed(input)
    } else {
        let mut output = Vec::with_capacity(input.len() + chars_to_escape);
        for &character in input.as_bytes() {
            if needs_escape(character) {
                output.push(ESCAPE_CHAR);
            }
            output.push(character);
        }
        let output = unsafe { String::from_utf8_unchecked(output) };
        Cow::Owned(output)
    }
}
