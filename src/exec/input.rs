use std::path::MAIN_SEPARATOR;
use std::borrow::Cow;

#[cfg(windows)]
const ESCAPE: char = '^';

#[cfg(not(windows))]
const ESCAPE: char = '\\';

/// A builder for efficiently generating input strings.
///
/// After choosing your required specs, the `get()` method will escape special characters found
/// in the input. Allocations will only occur if special characters are found that need to be
/// escaped.
pub struct Input<'a> {
    data: &'a str,
}

impl<'a> Input<'a> {
    /// Creates a new `Input` structure, which provides access to command-building
    /// primitives, such as `basename()` and `dirname()`.
    pub fn new(data: &'a str) -> Input<'a> {
        Input { data }
    }

    /// Removes the parent component of the path
    pub fn basename(&'a mut self) -> &'a mut Self {
        let mut index = 0;
        for (id, character) in self.data.char_indices() {
            if character == MAIN_SEPARATOR {
                index = id;
            }
        }

        if index != 0 {
            self.data = &self.data[index + 1..]
        }

        self
    }

    /// Removes the extension from the path
    pub fn remove_extension(&'a mut self) -> &'a mut Self {
        let mut dir_index = 0;
        let mut ext_index = 0;

        for (id, character) in self.data.char_indices() {
            if character == MAIN_SEPARATOR {
                dir_index = id;
            }
            if character == '.' {
                ext_index = id;
            }
        }

        // Account for hidden files and directories
        if ext_index != 0 && dir_index + 2 <= ext_index {
            self.data = &self.data[0..ext_index];
        }

        self
    }

    /// Removes the basename from the path.
    pub fn dirname(&'a mut self) -> &'a mut Self {
        let mut index = 0;
        for (id, character) in self.data.char_indices() {
            if character == MAIN_SEPARATOR {
                index = id;
            }
        }

        self.data = if index == 0 {
            "."
        } else {
            &self.data[0..index]
        };

        self
    }

    pub fn get(&'a self) -> Cow<'a, str> {
        fn char_is_quotable(x: char) -> bool {
            [
                ' ',
                '(',
                ')',
                '[',
                ']',
                '&',
                '$',
                '@',
                '{',
                '}',
                '<',
                '>',
                '|',
                ';',
                '"',
                '\'',
                '#',
                '*',
                '%',
                '?',
                '`',
            ].contains(&x)
        };

        // If a quotable character is found, we will use that position for allocating.
        let pos = match self.data.find(char_is_quotable) {
            Some(pos) => pos,
            // Otherwise, we will return the contents of `data` without allocating.
            None => return Cow::Borrowed(self.data),
        };

        // When building the input string, we will start by adding the characters that
        // we've already verified to be free of special characters.
        let mut owned = String::with_capacity(self.data.len());
        owned.push_str(&self.data[..pos]);
        owned.push(ESCAPE);

        // This slice contains the data that is left to be scanned for special characters.
        // If multiple characters are found, this slice will be sliced and updated multiple times.
        let mut slice = &self.data[pos..];

        // Repeatedly search for special characters until all special characters have been found,
        // appending and inserting the escape character each time, as well as updating our
        // starting position.
        while let Some(pos) = slice[1..].find(char_is_quotable) {
            owned.push_str(&slice[..pos + 1]);
            owned.push(ESCAPE);
            slice = &slice[pos + 1..];
        }

        // Finally, we return our newly-allocated input string.
        owned.push_str(slice);
        Cow::Owned(owned)
    }
}

#[cfg(test)]
mod tests {
    use super::{MAIN_SEPARATOR, Input};

    fn correct(input: &str) -> String {
        let mut sep = String::new();
        sep.push(MAIN_SEPARATOR);
        input.replace('/', &sep)
    }

    #[test]
    fn path_remove_ext_simple() {
        assert_eq!(&Input::new("foo.txt").remove_extension().get(), "foo");
    }

    #[test]
    fn path_remove_ext_dir() {
        assert_eq!(
            &Input::new(&correct("dir/foo.txt")).remove_extension().get(),
            &correct("dir/foo")
        );
    }

    #[test]
    fn path_hidden() {
        assert_eq!(&Input::new(".foo").remove_extension().get(), ".foo")
    }

    #[test]
    fn path_remove_ext_utf8() {
        assert_eq!(&Input::new("💖.txt").remove_extension().get(), "💖");
    }

    #[test]
    fn path_remove_ext_empty() {
        assert_eq!(&Input::new("").remove_extension().get(), "");
    }

    #[test]
    fn path_basename_simple() {
        assert_eq!(&Input::new("foo.txt").basename().get(), "foo.txt");
    }

    #[test]
    fn path_basename_dir() {
        assert_eq!(
            &Input::new(&correct("dir/foo.txt")).basename().get(),
            "foo.txt"
        );
    }

    #[test]
    fn path_basename_empty() {
        assert_eq!(&Input::new("").basename().get(), "");
    }

    #[test]
    fn path_basename_utf8() {
        assert_eq!(
            &Input::new(&correct("💖/foo.txt")).basename().get(),
            "foo.txt"
        );
        assert_eq!(
            &Input::new(&correct("dir/💖.txt")).basename().get(),
            "💖.txt"
        );
    }

    #[test]
    fn path_dirname_simple() {
        assert_eq!(&Input::new("foo.txt").dirname().get(), ".");
    }

    #[test]
    fn path_dirname_dir() {
        assert_eq!(&Input::new(&correct("dir/foo.txt")).dirname().get(), "dir");
    }

    #[test]
    fn path_dirname_utf8() {
        assert_eq!(
            &Input::new(&correct("💖/foo.txt")).dirname().get(),
            "💖"
        );
        assert_eq!(&Input::new(&correct("dir/💖.txt")).dirname().get(), "dir");
    }

    #[test]
    fn path_dirname_empty() {
        assert_eq!(&Input::new("").dirname().get(), ".");
    }

    #[cfg(windows)]
    #[test]
    fn path_special_chars() {
        assert_eq!(
            &Input::new("A Directory\\And A File").get(),
            "A^ Directory\\And^ A^ File"
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn path_special_chars() {
        assert_eq!(
            &Input::new("A Directory/And A File").get(),
            "A\\ Directory/And\\ A\\ File"
        );
    }
}
