## Contributing to *fd*

**Thank you very much for considering contributing to this project!**

We welcome any form of contribution:

  * New issues (feature requests, bug reports, questions, ideas, ...)
  * Pull requests (documentation improvements, code improvements, new features, ...)

**Note**: Before you take the time to open a pull request, please open a ticket first. This will
give us the chance to discuss any potential changes first.

## Pull Request Expectations

1. Your contribution should be high quality, and best effort. In particular, please don't make a Pull Request
   that was created using generative AI unless you have reviewed it yourself and understand what it does, and be
   able to meaningfully answer questions and make changes to it as needed.
2. Your code should successfully compile and pass tests before publishing a Pull Request. It's understandable
   if tests fail on one or two specific targets, but if the build doesn't pass for any targets, that is a signal
   that the PR is low effort.
3. It is your responsibility to ensure that the generated code does not violate any copyrights or patents. This is
   true of all contributions, but is especially relevant when using an LLM that may be generating output that is
   very similar or even identical to existing code with an incompatible license.
4. If an AI coding assistant or other LLM tool was used, you must indicate that an AI tool was used, and to what extent it was used.
5. AI should not be used to generate comments when communicating with maintainers. We expect communication with the maintainers,
   including PR descriptions and comments, to be from humans, not from an AI. Using an AI to assist with grammar, spelling, and
   proofreading or translating is acceptable, as long as the content and voice of the
   message is your own.

## Add an entry to the changelog

If your contribution changes the behavior of `fd` (as opposed to a typo-fix
in the documentation), please update the [`CHANGELOG.md`](CHANGELOG.md#upcoming-release) file
and describe your changes. This makes the release process much easier and
therefore helps to get your changes into a new `fd` release faster.

The top of the `CHANGELOG` contains an *"Upcoming release"* section with a few
subsections (Features, Bugfixes, …). Please add your entry to the subsection
that best describes your change.

Entries follow this format:
```
- Short description of what has been changed, see #123 (@user)
```
Here, `#123` is the number of the original issue and/or your pull request.
Please replace `@user` by your GitHub username.

## `--format` and metadata output

The `--format` option formats each search result using a template. Path placeholders are:

| Placeholder | Meaning |
| --- | --- |
| `{}` | Full path of the search result |
| `{/}` | File name (basename) |
| `{//}` | Parent directory |
| `{.}` | Full path without the extension |
| `{/.}` | File name without the extension |

Metadata placeholders are:

| Placeholder | Meaning |
| --- | --- |
| `{%y}` | File type: `file`, `dir`, `symlink`, or `other` |
| `{%s}` | File size in bytes |
| `{%n}` | File name (basename) |
| `{%p}` | File path |
| `{%t}` | Last modification time as `YYYY-MM-DD HH:MM:SS` with the local timezone |

Literal escapes in `--format` are interpreted as follows: `\\t` is a tab, `\\n` is a newline,
`\\r` is a carriage return, `\\0` is NUL, and `\\\\` is a literal backslash. Use `{{` and `}}`
for literal `{` and `}`. The `%` placeholders are only interpreted inside the `{...}`
tokens by `--format`; they remain literal in `--exec` and `--exec-batch`.

For clean output containing just the file name and size, use
`fd --format='{%n} {%s} bytes' pattern`.

For a readable standard table, use:

```bash
fd --format-metadata pattern
```

This prints a header and one row per result:

```text
Type | Size | Name | Path | Modified
file | 10376 bytes | exit.exe | target/.../exit.exe | 2026-09-10 14:35:27 CEST
```

For a custom layout, use:

```bash
fd --format='{%y} | {%s} bytes | {%n} | {%p} | {%t}' pattern
```

`--format-metadata` can be combined with `--format` to keep the standard
header while supplying a custom row layout:

```bash
fd --format-metadata --format='{%y} | {%s} bytes | {%n} | {%p} | {%t}' pattern
```

When changing this behavior, update the CLI help in `src/cli.rs`, the user-facing examples in
`README.md`, tests in `src/fmt/mod.rs` and `tests/tests.rs`, and add a `CHANGELOG.md` entry.

## Important links

  * [Open issues](https://github.com/sharkdp/fd/issues)
  * [Open pull requests](https://github.com/sharkdp/fd/pulls)
  * [Development section in the README](https://github.com/sharkdp/fd#development)
  * [fd on crates.io](https://crates.io/crates/fd-find)
  * [LICENSE-APACHE](https://github.com/sharkdp/fd/blob/master/LICENSE-APACHE) and [LICENSE-MIT](https://github.com/sharkdp/fd/blob/master/LICENSE-MIT)
