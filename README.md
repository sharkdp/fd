# fd
[![Build Status](https://travis-ci.org/sharkdp/fd.svg?branch=master)](https://travis-ci.org/sharkdp/fd)
[![Build status](https://ci.appveyor.com/api/projects/status/21c4p5fwggc5gy3j?svg=true)](https://ci.appveyor.com/project/sharkdp/fd)
[![Version info](https://img.shields.io/crates/v/fd-find.svg)](https://crates.io/crates/fd-find)

*fd* is a simple, fast and user-friendly alternative to
[*find*](https://www.gnu.org/software/findutils/).

While it does not seek to mirror all of *find*'s powerful functionality, it provides sensible
(opinionated) defaults for [80%](https://en.wikipedia.org/wiki/Pareto_principle) of the use cases.

## Features
* Convenient syntax: `fd PATTERN` instead of `find -iname '*PATTERN*'`.
* Colorized terminal output (similar to *ls*).
* It's *fast* (see benchmarks below).
* Smart case: the search is case-insensitive by default. It switches to
  case-sensitive if the pattern contains an uppercase
  character[\*](http://vimdoc.sourceforge.net/htmldoc/options.html#'smartcase').
* Ignores hidden directories and files, by default.
* Ignores patterns from your `.gitignore`, by default.
* Regular expressions.
* Unicode-awareness.
* The command name is *50%* shorter[\*](https://github.com/ggreer/the_silver_searcher) than
  `find` :-).

## Demo

![Demo](http://i.imgur.com/kTMFSVU.gif)

## Colorized output
`fd` can colorize files by extension, just like `ls`. In order for this to work, the environment
variable [`LS_COLORS`](https://linux.die.net/man/5/dir_colors) has to be set. Typically, the value
of this variable is set by the `dircolors` command which provides a convenient configuration format
to define colors for different file formats.
On most distributions, `LS_COLORS` should be set already. If you are looking for alternative, more
complete (and more colorful) variants, see
[here](https://github.com/seebi/dircolors-solarized) or
[here](https://github.com/trapd00r/LS_COLORS).

## Benchmark
Let's search my home folder for files that end in `[0-9].jpg`. It contains ~150.000
subdirectories and about a million files. For averaging and statistical analysis, I'm using
[bench](https://github.com/Gabriel439/bench). All benchmarks are performed for a "warm
cache". Results for a cold cache are similar.

Let's start with `find`:
```
find ~ -iregex '.*[0-9]\.jpg$'

time                 6.265 s    (6.127 s .. NaN s)
                     1.000 R²   (1.000 R² .. 1.000 R²)
mean                 6.162 s    (6.140 s .. 6.181 s)
std dev              31.73 ms   (0.0 s .. 33.48 ms)
```

`find` is much faster if it does not need to perform a regular-expression search:
```
find ~ -iname '*[0-9].jpg'

time                 2.866 s    (2.754 s .. 2.964 s)
                     1.000 R²   (0.999 R² .. 1.000 R²)
mean                 2.860 s    (2.834 s .. 2.875 s)
std dev              23.11 ms   (0.0 s .. 25.09 ms)
```

Now let's try the same for `fd`. Note that `fd` *always* performs a regular expression
search. The options `--hidden` and `--no-ignore` are needed for a fair comparison,
otherwise `fd` does not have to traverse hidden folders and ignored paths (see below):
```
fd --hidden --no-ignore '.*[0-9]\.jpg$' ~

time                 892.6 ms   (839.0 ms .. 915.4 ms)
                     0.999 R²   (0.997 R² .. 1.000 R²)
mean                 871.2 ms   (857.9 ms .. 881.3 ms)
std dev              15.50 ms   (0.0 s .. 17.49 ms)
```
For this particular example, `fd` is approximately seven times faster than `find -iregex`
and about three times faster than `find -iname`. By the way, both tools found the exact
same 14030 files :smile:.

Finally, let's run `fd` without `--hidden` and `--no-ignore` (this can lead to different
search results, of course):
```
fd '[0-9]\.jpg$' ~

time                 159.5 ms   (155.8 ms .. 165.3 ms)
                     0.999 R²   (0.996 R² .. 1.000 R²)
mean                 158.7 ms   (156.5 ms .. 161.6 ms)
std dev              3.263 ms   (2.401 ms .. 4.298 ms)
```

**Note**: This is *one particular* benchmark on *one particular* machine. While I have
performed quite a lot of different tests (and found consistent results), things might
be different for you! I encourage everyone to try it out on their own.

Concerning *fd*'s speed, the main credit goes to the `regex` and `ignore` crates that are also used
in [ripgrep](https://github.com/BurntSushi/ripgrep) (check it out!).

## Install
With Rust's package manager [cargo](https://github.com/rust-lang/cargo), you can install *fd* via:
```
cargo install fd-find
```
Note that rust version *1.16.0* or later is required.
The release page of this repository also includes precompiled binaries for Linux.

On **macOS**, you can use [Homebrew](http://braumeister.org/formula/fd):
```
brew install fd
```

On **Arch Linux**, you can install the AUR package [fd-rs](https://aur.archlinux.org/packages/fd-rs/) via yaourt, or manually:
```
git clone https://aur.archlinux.org/fd-rs.git
cd fd-rs
makepkg -si
```

## Development
```bash
git clone https://github.com/sharkdp/fd

# Build
cd fd
cargo build

# Run unit tests
cargo test

# Run integration tests
cd tests
bash test.sh

# Install
cargo install
```

## Command-line options
```
USAGE:
    fd [FLAGS/OPTIONS] [<pattern>] [<path>]

FLAGS:
    -s, --case-sensitive    Case-sensitive search (default: smart case)
    -p, --full-path         Search full path (default: file-/dirname only)
    -H, --hidden            Search hidden files and directories
    -I, --no-ignore         Do not respect .(git)ignore files
    -L, --follow            Follow symlinks
    -0, --print0            Separate results by the null character
    -a, --absolute-path     Show absolute instead of relative paths
    -n, --no-color          Do not colorize output
    -h, --help              Prints help information
    -V, --version           Prints version information

OPTIONS:
    -d, --max-depth <depth>                  Set maximum search depth (default: none)
    -j, --threads <threads>                  The number of threads used for searching
    -t, --type <file-type>                   The type of file to search for [values: f, file, d, directory, s, symlink]
    -e, --file-extension <file-extension>    Search for a file extension instead of a pattern (option)

ARGS:
    <pattern>    the search pattern, a regular expression (optional)
    <path>       the root directory for the filesystem search (optional)

```

## Examples

First to get `fd`'s help run: 

```
fd --help
```

Let's assume we have some files we need to search through like so: 

```
fd_examples
├── .gitignore
├── not_file
├── sub_dir
│   ├── .here_be_tests
│   ├── more_dir
│   │   ├── .not_here
│   │   ├── even_further_down
│   │   │   ├── not_me.sh
│   │   │   ├── test_seven
│   │   │   └── testing_eight
│   │   ├── not_file -> /Users/fd_user/fd_examples/not_file
│   │   └── test_file_six
│   ├── test_file_five
│   ├── test_file_four
│   └── test_file_three
├── test_file_one
├── test_file_two
├── test_one
└── this_is_a_test
```

Let's do a recursive search for anything that has the name test in it (`fd` will start in the current directory and follow symbolic links by default):

`fd test`

This will return us: 
```
sub_dir/more_dir/even_further_down/test_seven
sub_dir/more_dir/even_further_down/testing_eight
sub_dir/more_dir/test_file_six
sub_dir/test_file_five
sub_dir/test_file_three
sub_dir/test_four
test_file_one
test_file_two
test_one
this_is_a_test
```

As you can see this matched every file that had 'test' as a substring in it. Whoops! We forgot a hidden file (`.here_be_tests`) Lets get that by telling `fd` we want to see all files:

`fd --hidden 'test'`

There they all are:
```
sub_dir/.here_be_tests
sub_dir/more_dir/even_further_down/test_seven
sub_dir/more_dir/even_further_down/testing_eight
sub_dir/more_dir/test_file_six
sub_dir/test_file_five
sub_dir/test_file_four
sub_dir/test_file_three
test_file_one
test_file_two
test_one
this_is_a_test
```

What if we wanted to find only when the file began with "test"? Well, `fd` does regex searches (by default) so using the regex indicator for beginning of line `^` will get us what we want: 

`fd '^test'`

Giving us: 
```
sub_dir/more_dir/even_further_down/test_seven
sub_dir/more_dir/even_further_down/testing_eight
sub_dir/more_dir/test_file_six
sub_dir/test_file_five
sub_dir/test_file_three
sub_dir/test_four
test_file_one
test_file_two
test_one
```

What if I only wanted to see the file with `test` in them only in the `fd_examples/sub_dir` folder? I Can do this from anywhere in the file structure by giving it the path I would like it to search:

`fd test ~/fd_examples/sub_dir/`

```
fd_user on Falcon in ~/Desktop
> fd test ~/fd_examples/sub_dir/
/Users/fd_user/fd_examples/sub_dir/more_dir/even_further_down/test_seven
/Users/fd_user/fd_examples/sub_dir/more_dir/even_further_down/testing_eight
/Users/fd_user/fd_examples/sub_dir/more_dir/test_file_six
/Users/fd_user/fd_examples/sub_dir/test_file_five
/Users/fd_user/fd_examples/sub_dir/test_file_three
/Users/fd_user/fd_examples/sub_dir/test_four
```

If we don't give `fd` and argument it will recursively search the current directory for all files (like `ls -R`): 

```
not_file
sub_dir
sub_dir/more_dir
sub_dir/more_dir/even_further_down
sub_dir/more_dir/even_further_down/test_seven
sub_dir/more_dir/even_further_down/testing_eight
sub_dir/more_dir/not_file
sub_dir/more_dir/test_file_six
sub_dir/test_file_five
sub_dir/test_file_three
sub_dir/test_four
test_file_one
test_file_two
test_one
this_is_a_test
```

`fd` is magic, it will look for a `.gitignore` file and treat the rules inside it as rules in the search pattern. So if we have a `.gitignore` file like: 

```
*.sh
```
`fd` will then never look for any files that end in `.sh`. We can tell `fd` to ignore `.gitignore` files with `-I` (or `--ignore`) we can temporarliy stop that from happening: 

`fd -I me`

```
sub_dir/more_dir/even_further_down/not_me.sh
```

Searching for a file extension is easy too, don't forget your regex rules (`$` is the end of a line): 

`fd -I '\.sh$'`

```
sub_dir/more_dir/even_further_down/not_me.sh
```

What if we wanted to run some complicated bash follow on to the files? `xargs` can help us with that: 

`fd -0 'test' | xargs -0 -I {} cp {} {}.new`

In the example there's a couple things to make note:
  - First we are telling `fd` we want a null character to seperate the files `-0`, this is important when passing to `xargs`
  - second we are piping the output to `xargs` and telling this program to expect input null terminated with `-0` (the same syntax that `fd` was built with).
  - Then for fun are are using `-I` to replace a string `{}` and lauching `cp` to copy the file `{}` to a file ending in `{}.new` 

That gives us: 

```
.
├── .gitignore
├── not_file
├── sub_dir
│   ├── .here_be_tests
│   ├── more_dir
│   │   ├── .not_here
│   │   ├── even_further_down
│   │   │   ├── not_me.sh
│   │   │   ├── test_seven
│   │   │   ├── test_seven.new
│   │   │   ├── testing_eight
│   │   │   └── testing_eight.new
│   │   ├── not_file -> /Users/fd_user/fd_examples/not_file
│   │   ├── test_file_six
│   │   └── test_file_six.new
│   ├── test_file_five
│   ├── test_file_five.new
│   ├── test_file_four
│   ├── test_file_four.new
│   ├── test_file_three
│   └── test_file_three.new
├── test_file_one
├── test_file_one.new
├── test_file_two
├── test_file_two.new
├── test_one
├── test_one.new
├── this_is_a_test
└── this_is_a_test.new
```

`fd` can also show us the absolute path vs. the full path: 

`fd -a 'new'`

```
/Users/fd_user/fd_examples/sub_dir/more_dir/even_further_down/test_seven.new
/Users/fd_user/fd_examples/sub_dir/more_dir/even_further_down/testing_eight.new
/Users/fd_user/fd_examples/sub_dir/more_dir/test_file_six.new
/Users/fd_user/fd_examples/sub_dir/test_file_five.new
/Users/fd_user/fd_examples/sub_dir/test_file_four.new
/Users/fd_user/fd_examples/sub_dir/test_file_three.new
/Users/fd_user/fd_examples/test_file_one.new
/Users/fd_user/fd_examples/test_file_two.new
/Users/fd_user/fd_examples/test_one.new
/Users/fd_user/fd_examples/this_is_a_test.new
```

Be careful when using the full path flag (`--full-path`), as this searches the **path** of the file as well so regex like `^` won't work as intended: 

```
fd_user on Falcon in ~/fd_examples
> fd --full-path '^test' ~/fd_examples/

```

The new feature `-e` for file extensions does the same as the regex `\.<extension>$` looking for the file `not_me.sh` would be done by:

`fd -HI -e sh not_me`

This will look for the pattern `not_me` with the ending extension `.sh`! You can always use the `-e` extension with an empty pattern to look for any file types like: 

`fd -e rs` 
