# Draft: PR #1917 Re-architecture per Reviewer Feedback

## Requirements (confirmed)
- Re-architect `--full-path` path resolution to cache `env::current_dir()` once at startup
- Fail early if cwd retrieval fails (before walk starts)
- Use cached cwd for infallible absolute path construction during walk
- Remove `FatalError` variant from `WorkerResult` (no longer needed)
- Revert all FatalError-related changes in walk.rs, job.rs
- Revert `batch()` to original `filter_map` style
- Rewrite test to avoid `env::set_current_dir()` (process-global state)
- Reply to all 6 review comments on GitHub
- Push updated changes to PR branch

## Technical Decisions
- **Config field**: `cwd: Option<PathBuf>` - `Some(cwd)` when `search_full_path` is true, `None` otherwise
- **New function**: `filesystem::make_absolute(path, cwd) -> PathBuf` - infallible, used by search_str_for_entry
- **Keep existing**: `path_absolute_form()` stays unchanged (used by hyperlinks, --absolute-path flag)
- **search_str_for_entry signature**: Changes from `(path, bool) -> io::Result<Cow>` to `(path, Option<&Path>) -> Cow` (infallible)
- **Early failure**: In `construct_config()`, call `env::current_dir()` when `opts.full_path` is true. Return error with context if it fails.

## Research Findings
- `path_absolute_form` is also used by `absolute_path()` in filesystem.rs, which is called from cli.rs (--absolute-path flag) and hyperlink.rs. These callers are independent and unaffected.
- `execute_batch` takes `I: Iterator<Item = PathBuf>` - compatible with filter_map return
- Config is constructed AFTER `set_working_dir()` runs, so cwd is already set to --base-directory at that point
- Tests compile and run: `cargo test --no-run` succeeds
- PR has 2 commits: the fix commit + a CI retrigger commit

## Scope Boundaries
- INCLUDE: config.rs, main.rs, filesystem.rs, walk.rs, exec/job.rs, CHANGELOG.md, tests
- INCLUDE: GitHub comment replies, push to PR branch
- EXCLUDE: Changes to path_absolute_form, absolute_path, hyperlink.rs, cli.rs
- EXCLUDE: Integration tests (cargo test covers existing ones)

## Reviewer Comments to Address
1. Top-level: Re-architecture suggestion → Implementing this
2. job.rs:57: Vec allocation in batch → Reverting to filter_map
3. walk.rs:238: Duplicated error handling → Removing FatalError entirely
4. walk.rs:234: Fatal errors should always print → FatalError gone, early failure instead
5. walk.rs:542: Unnecessary match → FatalError gone
6. walk.rs:729: Test uses process-global set_current_dir → Rewriting test
