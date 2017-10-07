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

On **Windows**, you can pre-built binaries from the [Release page](https://github.com/sharkdp/fd/releases).

## Development
```bash
git clone https://github.com/sharkdp/fd

# Build
cd fd
cargo build

# Run unit tests and integration tests
cargo test

# Install
cargo install
```

## Command-line options
```
USAGE:
    fd [FLAGS/OPTIONS] [<pattern>] [<path>]

FLAGS:
    -H, --hidden            Search hidden files and directories
    -I, --no-ignore         Do not respect .(git)ignore files
    -s, --case-sensitive    Case-sensitive search (default: smart case)
    -a, --absolute-path     Show absolute instead of relative paths
    -L, --follow            Follow symbolic links
    -p, --full-path         Search full path (default: file-/dirname only)
    -0, --print0            Separate results by the null character
    -h, --help              Prints help information
    -V, --version           Prints version information

OPTIONS:
    -d, --max-depth <depth>    Set maximum search depth (default: none)
    -t, --type <filetype>      Filter by type: f(ile), d(irectory), s(ymlink)
    -e, --extension <ext>      Filter by file extension
    -c, --color <when>         When to use color in the output:
                               never, auto, always (default: auto)
    -j, --threads <num>        Set number of threads to use for searching:
                               (default: number of available CPU cores)

ARGS:
    <pattern>    the search pattern, a regular expression (optional)
    <path>       the root directory for the filesystem search (optional)
```

## Tutorial

First, to see all command line options, you can get `fd`'s help text by running:
```
fd --help
```

For the sake of this tutorial, let's assume we have a directory with the following file structure:
```
fd_examples
├── .gitignore
├── desub_dir
│   └── old_test.txt
├── not_file
├── sub_dir
│   ├── .here_be_tests
│   ├── more_dir
│   │   ├── .not_here
│   │   ├── even_further_down
│   │   │   ├── not_me.sh
│   │   │   ├── test_seven
│   │   │   └── testing_eight
│   │   ├── not_file -> /Users/fd_user/Desktop/fd_examples/not_file
│   │   └── test_file_six
│   ├── new_test.txt
│   ├── test_file_five
│   ├── test_file_four
│   └── test_file_three
├── test_file_one
├── test_file_two
├── test_one
└── this_is_a_test
```

If `fd` is called with a single argument (the search pattern), it will perform a recursive search
through the current directory. To search for all files that include the string "test", we can
simply run:
```
> fd test
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

The search pattern is treated as a regular expression. To show only entries that start with "test",
we can simply run:
```
> fd '^test'
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

Note that `fd` does not show hidden files (`.here_be_tests`) by default. To change this, we can use
the `-H` (or `--hidden`) option:
```
> fd -H test
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

If we are interested in showing the results from a particular directory, we can specify the root of
the search as a second argument:
```
> fd test sub_dir
sub_dir/more_dir/even_further_down/test_seven
sub_dir/more_dir/even_further_down/testing_eight
sub_dir/more_dir/test_file_six
sub_dir/test_file_five
sub_dir/test_file_three
sub_dir/test_four
```

If we don't give *any* arguments to `fd`, it simply shows all entries in the current directory,
recursively (like `ls -R`):
```
> fd
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

If we work in a directory that is a Git repository (or includes several Git repositories), `fd`
does not search folders (and does not show files) that match the `.gitignore` pattern. For example,
imagine we had a `.gitignore` file with the following content:
```
*.sh
```
In this case, `fd` would not show any files that end in `.sh`. To disable this behavior, we can
use the `-I` (or `--ignore`) option:
```
> fd -I me
sub_dir/more_dir/even_further_down/not_me.sh
```

To really search *all* files and directories, we can combine the hidden and ignore features to show
everything (`-HI`):
```
fd -HI 'not|here'
not_file
sub_dir/.here_be_tests
sub_dir/more_dir/.not_here
sub_dir/more_dir/even_further_down/not_me.sh
sub_dir/more_dir/not_file
```

Searching for a file extension is easy too, using the `-e` (or `--file-extensions`) switch for file
extensions:
```
> fd -e sh
sub_dir/more_dir/even_further_down/not_me.sh
```

Next, we can even use a pattern in combination with `-e` to search for a regex pattern over the
files that end in the specified extension.
```
> fd -e txt test
fd_examples/desub_dir/old_test.txt
fd_examples/sub_dir/new_test.txt
```

What if we wanted to run a command for each of the search results? We can use `xargs` to do that:
`fd -0 'test' | xargs -0 -I {} cp {} {}.new`

In this example there are a couple things to take note:
  - First we are telling `fd` we want a null character to seperate the files `-0`, this is
    important when passing to `xargs`.
  - Second, we are piping the output to `xargs` and telling this program to expect input null
    terminated with `-0` (the same syntax that `fd` was built with).
  - Then for fun we are using `-I` to replace a string `{}` and lauching `cp` to copy the file `{}`
    to a file ending in `{}.new`.

`fd` can also show us the absolute path vs. the full path with `-a` (`--absolute-path`):
```
> fd -a new
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
