# Re-architect PR #1917: Cache cwd at Startup for --full-path

## TL;DR

> **Quick Summary**: Rewrite the `--full-path` path resolution in sharkdp/fd PR #1917 per reviewer tmccombs' feedback: cache `env::current_dir()` once at startup instead of calling it per-entry, remove the `FatalError` error-handling machinery entirely, reply to all review comments, and push.
>
> **Deliverables**:
> - Re-architected cwd caching in Config struct
> - Infallible `make_absolute()` function in filesystem.rs
> - Simplified `search_str_for_entry()` (no more `Result`)
> - `FatalError` variant removed from `WorkerResult` + all consumers
> - `batch()` reverted to original `filter_map` pattern
> - Rewritten unit test (no process-global state)
> - Replies posted to all 6 review comments
> - Changes pushed to PR branch
>
> **Estimated Effort**: Medium
> **Parallel Execution**: YES - 3 waves
> **Critical Path**: Task 1 → Task 3 → Task 5 → Task 7

---

## Context

### Original Request
PR #1917 on sharkdp/fd fixes issue #1900 where `fd --full-path` panics when the cwd becomes invalid during execution. The current implementation wraps `current_dir()` calls in error handling via a new `FatalError` variant. Reviewer tmccombs (project collaborator) requested CHANGES_REQUESTED with a fundamental re-architecture: cache cwd once at startup, fail early if needed, use it infallibly thereafter.

### Interview Summary
**Key Discussions**:
- The reviewer's approach is clearly better: no per-entry syscalls, no `FatalError` machinery, simpler code
- `path_absolute_form()` stays unchanged — it's used by hyperlinks and `--absolute-path`, which are independent call paths
- `Config` is constructed AFTER `set_working_dir()` in `run()` (line 82 before line 101), so cached cwd correctly reflects `--base-directory`
- `execute_batch` already accepts `I: Iterator<Item = PathBuf>`, so the `filter_map` revert works cleanly

**Research Findings**:
- `path_absolute_form` callers: `search_str_for_entry` (changing), `absolute_path` → `cli.rs:706` + `hyperlink.rs:9` (untouched)
- Master's `batch()` used `filter_map` directly into `execute_batch` — no Vec allocation. Current PR broke this.
- Master's `job()` had no `FatalError` arm — just `Entry` and `Error`
- Master's `WorkerResult` had 2 variants: `Entry`, `Error`. PR added `FatalError` as 3rd.
- Master's `search_str_for_entry` didn't exist as a function — the logic was inlined with `.expect()` (the panic source)

### Self-Review Gap Analysis
**Gaps identified and resolved**:
- Edge case: `--full-path` + `--base-directory` combo → Safe. `set_working_dir()` runs before `construct_config()`, so cached cwd reflects the base directory. ✅
- Edge case: absolute search paths with `--full-path` → `make_absolute` returns path as-is when already absolute. ✅
- Edge case: `--full-path` NOT set + invalid cwd → No problem. `cwd` is `None`, `search_str_for_entry` uses filename only. ✅
- `./` prefix stripping: Current `path_absolute_form` strips `./` prefix. New `make_absolute` must do the same. ✅
- Import cleanup: Removing `FatalError` means `io` import in walk.rs `search_str_for_entry` is no longer needed for the `io::Result` return type. Must verify. ✅

---

## Work Objectives

### Core Objective
Replace per-entry `current_dir()` calls with a single cached cwd at startup, eliminating all `FatalError` error-handling machinery and making `--full-path` path resolution infallible during the walk.

### Concrete Deliverables
- `src/config.rs`: New `cwd: Option<PathBuf>` field
- `src/main.rs`: `construct_config()` populates `cwd` with early failure
- `src/filesystem.rs`: New `make_absolute(path, cwd)` function + unit tests
- `src/walk.rs`: Infallible `search_str_for_entry`, no `FatalError` in enum or receiver
- `src/exec/job.rs`: `FatalError` arms removed, `batch()` reverted to `filter_map`
- GitHub: Replies on all 6 review comments
- Git: Changes committed and pushed to `origin/issue-1900`

### Definition of Done
- [ ] `cargo build` succeeds with no errors
- [ ] `cargo test` passes all tests
- [ ] `cargo clippy` reports no warnings
- [ ] All 6 review comments replied to on GitHub
- [ ] Changes pushed to PR branch

### Must Have
- Cached cwd at startup (in Config), not per-entry
- Early failure with descriptive error if `--full-path` and cwd is invalid
- No `FatalError` variant anywhere in the codebase
- `batch()` uses `filter_map` (no Vec allocation)
- Unit test for `make_absolute` that doesn't use process-global state
- `./` prefix stripping preserved in new function (matching `path_absolute_form` behavior)

### Must NOT Have (Guardrails)
- Do NOT modify `path_absolute_form()` or `absolute_path()` in filesystem.rs — other callers depend on them
- Do NOT touch `hyperlink.rs` or `cli.rs`
- Do NOT add new error handling machinery (the whole point is to remove it)
- Do NOT use `env::set_current_dir()` in new tests (process-global, affects all threads)
- Do NOT use AI-slop language in GitHub replies (no "Great suggestion!", "I appreciate your feedback", "Absolutely!", "This is a fantastic idea")
- Do NOT introduce new abstractions beyond what's needed (no traits, no new error types)

---

## Verification Strategy

### Test Decision
- **Infrastructure exists**: YES (cargo test, unit tests in-file, integration tests in tests/tests.rs)
- **User wants tests**: YES (unit tests for new function)
- **Framework**: Rust built-in `#[test]` + tempfile crate (already a dev dependency)

### Approach
- Unit tests for `make_absolute` in `filesystem.rs::tests` (pure function, no side effects)
- Delete the old `walk.rs` test that used `env::set_current_dir()`
- Existing integration tests in `tests/tests.rs` validate `--full-path` behavior end-to-end
- `cargo test` runs everything

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately):
├── Task 1: Config + construct_config changes (config.rs, main.rs)
└── Task 2: make_absolute function + unit tests (filesystem.rs)

Wave 2 (After Wave 1):
└── Task 3: Core re-architecture (walk.rs, job.rs) — depends on 1, 2

Wave 3 (After Wave 2):
├── Task 4: Test cleanup + CHANGELOG (walk.rs, CHANGELOG.md)
└── Task 5: Build verification (cargo build/test/clippy)

Wave 4 (After Wave 3):
├── Task 6: Reply to review comments (GitHub)
└── Task 7: Commit + push (git)

Critical Path: Task 1 → Task 3 → Task 5 → Task 7
Parallel Speedup: ~30% faster than fully sequential
```

### Dependency Matrix

| Task | Depends On | Blocks | Can Parallelize With |
|------|------------|--------|---------------------|
| 1 | None | 3 | 2 |
| 2 | None | 3 | 1 |
| 3 | 1, 2 | 4, 5 | None |
| 4 | 3 | None | 5 |
| 5 | 3 | 7 | 4, 6 |
| 6 | None (content known) | None | 4, 5 |
| 7 | 5 | None | 6 |

### Agent Dispatch Summary

| Wave | Tasks | Recommended Dispatch |
|------|-------|---------------------|
| 1 | 1, 2 | Parallel: both are independent file edits |
| 2 | 3 | Sequential: modifies walk.rs + job.rs together |
| 3 | 4, 5 | 4 is quick cleanup; 5 is verify (run after 4) |
| 4 | 6, 7 | 6 can start during 5; 7 after 5 confirms clean |

---

## TODOs

- [ ] 1. Add `cwd` field to Config + populate in construct_config

  **What to do**:

  **Step 1a — config.rs**: Add a new field to the `Config` struct:
  ```rust
  /// Cached current working directory for absolute path construction.
  /// Only populated when `search_full_path` is true.
  pub cwd: Option<PathBuf>,
  ```
  Place it near the `search_full_path` field (after line 20) for logical grouping.

  **Step 1b — main.rs**: In `construct_config()`, before the `Ok(Config { ... })` block, compute the cwd:
  ```rust
  let cwd = if opts.full_path {
      Some(env::current_dir().context(
          "Could not determine current directory. \
           This is required for --full-path."
      )?)
  } else {
      None
  };
  ```
  Then add `cwd,` to the Config struct literal (around line 250, near `search_full_path`).

  **Must NOT do**:
  - Do not call `current_dir()` unconditionally — only when `full_path` is true
  - Do not add cwd caching logic anywhere else (not in walk.rs, not in filesystem.rs)

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`git-master`]
    - `git-master`: For clean commit at the end

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Task 2)
  - **Blocks**: Task 3
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/config.rs:14-136` — Config struct definition. Add new field with doc comment following existing style (see `search_full_path` at line 20 for placement)
  - `src/main.rs:195-331` — `construct_config()` function. The cwd computation goes before the `Ok(Config { ... })` block. Note how other fallible computations like `time_constraints` (line 214) use `?` for early failure.

  **API/Type References**:
  - `std::env::current_dir()` — returns `io::Result<PathBuf>`. Wrap with `.context()` for descriptive error.
  - `anyhow::Context` — already imported at main.rs:21 (`use anyhow::{Context, Result, anyhow, bail}`)

  **Flow References**:
  - `src/main.rs:74-111` — `run()` function. Line 82 calls `set_working_dir(&opts)?` BEFORE line 101 calls `construct_config(opts, &pattern_regexps)?`. This ordering guarantees the cached cwd reflects any `--base-directory` setting.

  **Acceptance Criteria**:
  ```bash
  # Verify it compiles (walk.rs will have errors until Task 3, but config.rs and main.rs should be clean):
  cargo check 2>&1 | grep -E "^error" | head -5
  # Expected: errors only from walk.rs/job.rs (FatalError still referenced there), NOT from config.rs or main.rs
  ```

  **Commit**: NO (groups with Task 7)

---

- [ ] 2. Add `make_absolute` function to filesystem.rs with unit tests

  **What to do**:

  **Step 2a — New function**: Add after `path_absolute_form` (after line 21):
  ```rust
  /// Construct an absolute path from a potentially relative path and a
  /// pre-resolved working directory. Unlike `path_absolute_form`, this
  /// does not call `env::current_dir()` and cannot fail.
  pub fn make_absolute(path: &Path, cwd: &Path) -> PathBuf {
      if path.is_absolute() {
          return path.to_path_buf();
      }
      let path = path.strip_prefix(".").unwrap_or(path);
      cwd.join(path)
  }
  ```

  **Step 2b — Unit tests**: Add to the existing `mod tests` block (after line 155):
  ```rust
  #[test]
  fn make_absolute_with_relative_path() {
      use super::make_absolute;
      use std::path::PathBuf;

      let cwd = Path::new("/home/user");
      assert_eq!(
          make_absolute(Path::new("foo/bar"), cwd),
          PathBuf::from("/home/user/foo/bar")
      );
  }

  #[test]
  fn make_absolute_strips_dot_prefix() {
      use super::make_absolute;
      use std::path::PathBuf;

      let cwd = Path::new("/home/user");
      assert_eq!(
          make_absolute(Path::new("./foo/bar"), cwd),
          PathBuf::from("/home/user/foo/bar")
      );
  }

  #[test]
  fn make_absolute_with_absolute_path() {
      use super::make_absolute;
      use std::path::PathBuf;

      let cwd = Path::new("/home/user");
      assert_eq!(
          make_absolute(Path::new("/absolute/path"), cwd),
          PathBuf::from("/absolute/path")
      );
  }
  ```

  **Must NOT do**:
  - Do not modify `path_absolute_form` or `absolute_path` functions
  - Do not use `env::set_current_dir` or any process-global state in tests
  - Do not call `env::current_dir()` in the new function — the whole point is it takes cwd as a parameter

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Task 1)
  - **Blocks**: Task 3
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/filesystem.rs:14-21` — `path_absolute_form`. The new function mirrors this logic exactly but takes `cwd: &Path` instead of calling `env::current_dir()`. Note the `strip_prefix(".")` on line 19 — MUST preserve this behavior.
  - `src/filesystem.rs:139-155` — Existing test module. Follow the same style (imports at top of test block, `assert_eq!` with Path/PathBuf).

  **Acceptance Criteria**:
  ```bash
  cargo test filesystem::tests -- --nocapture 2>&1
  # Expected: 3 new tests pass (make_absolute_with_relative_path, make_absolute_strips_dot_prefix, make_absolute_with_absolute_path)
  # Plus existing strip_current_dir_basic still passes
  ```

  **Commit**: NO (groups with Task 7)

---

- [ ] 3. Core re-architecture: rewrite search_str_for_entry + remove FatalError entirely

  **What to do**:

  This is the main task. It touches walk.rs and job.rs to remove all FatalError infrastructure and make path resolution infallible.

  **Step 3a — Rewrite `search_str_for_entry` (walk.rs lines 686-703)**: Change from fallible (returns `io::Result`) to infallible. New signature takes `Option<&Path>` instead of `bool`:
  ```rust
  fn search_str_for_entry<'a>(
      entry_path: &'a std::path::Path,
      cwd: Option<&std::path::Path>,
  ) -> Cow<'a, OsStr> {
      if let Some(cwd) = cwd {
          let abs_path = filesystem::make_absolute(entry_path, cwd);
          Cow::Owned(abs_path.as_os_str().to_os_string())
      } else {
          match entry_path.file_name() {
              Some(filename) => Cow::Borrowed(filename),
              None => unreachable!(
                  "Encountered file system entry without a file name. This should only \
                   happen for paths like 'foo/bar/..' or '/' which are not supposed to \
                   appear in a file system traversal."
              ),
          }
      }
  }
  ```

  **Step 3b — Simplify the call site in `spawn_senders` (walk.rs lines 534-547)**: Replace the match-with-error-handling with a direct call:
  ```rust
  let search_str = search_str_for_entry(entry_path, config.cwd.as_deref());
  ```
  Remove the entire `Err(err) => { ... FatalError ... WalkState::Quit }` block.

  **Step 3c — Remove `FatalError` from `WorkerResult` enum (walk.rs lines 40-46)**: Delete the `FatalError(ignore::Error)` variant. The enum goes back to just `Entry` and `Error`.

  **Step 3d — Remove `FatalError` arm from `ReceiverBuffer::poll()` (walk.rs lines 233-238)**: Delete the entire block:
  ```rust
  WorkerResult::FatalError(err) => {
      if self.config.show_filesystem_errors {
          print_error(err.to_string());
      }
      return Err(ExitCode::GeneralError);
  }
  ```

  **Step 3e — Remove `FatalError` arm from `job()` (job.rs lines 31-36)**: Delete:
  ```rust
  WorkerResult::FatalError(err) => {
      if config.show_filesystem_errors {
          print_error(err.to_string());
      }
      return ExitCode::GeneralError;
  }
  ```

  **Step 3f — Revert `batch()` to filter_map (job.rs lines 52-81)**: Replace the entire function body with the master version:
  ```rust
  pub fn batch(
      results: impl IntoIterator<Item = WorkerResult>,
      cmd: &CommandSet,
      config: &Config,
  ) -> ExitCode {
      let paths = results
          .into_iter()
          .filter_map(|worker_result| match worker_result {
              WorkerResult::Entry(dir_entry) => Some(dir_entry.into_stripped_path(config)),
              WorkerResult::Error(err) => {
                  if config.show_filesystem_errors {
                      print_error(err.to_string());
                  }
                  None
              }
          });

      cmd.execute_batch(paths, config.batch_size, config.path_separator.as_deref())
  }
  ```

  **Step 3g — Clean up imports**: After removing FatalError and making search_str_for_entry infallible:
  - `walk.rs`: The `io` import (line 3: `use std::io::{self, Write}`) may need adjustment — `io::Result` is no longer used by `search_str_for_entry`, but `io::Write` is still used by `ReceiverBuffer`. Verify `io` is still needed (it is, for `Write` and `io::stdout()` and `io::BufWriter`).
  - `walk.rs`: Remove `use std::path::PathBuf` from the `search_str_for_entry` error path if it was only used there (check: `PathBuf` is used in other places like `scan()` signature, so it stays).
  - `job.rs`: Verify no dead imports after removing FatalError arm.

  **Must NOT do**:
  - Do not modify `path_absolute_form` or `absolute_path`
  - Do not change any other part of `spawn_senders` beyond the `search_str` computation
  - Do not add new error variants or error handling
  - Do not change the ReceiverBuffer's `WorkerResult::Error` handling (that stays as-is)

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`git-master`]
    - `git-master`: For clean commit at the end

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2 (sequential)
  - **Blocks**: Tasks 4, 5
  - **Blocked By**: Tasks 1, 2

  **References**:

  **Pattern References**:
  - `src/walk.rs:686-703` — Current `search_str_for_entry` (to be rewritten). Note the `io::Result` return and `path_absolute_form` call.
  - `src/walk.rs:534-547` — Current call site with FatalError error handling (to be simplified to one line).
  - `src/walk.rs:40-46` — `WorkerResult` enum with FatalError variant (to be reduced to 2 variants).
  - `src/walk.rs:233-238` — FatalError arm in ReceiverBuffer (to be deleted).
  - `src/exec/job.rs:31-36` — FatalError arm in job() (to be deleted).
  - `src/exec/job.rs:52-81` — Current batch() with Vec (to be reverted to filter_map).

  **Master Reference (what to revert toward)**:
  - Run `git show master:src/exec/job.rs` to see original batch() with filter_map pattern.
  - Run `git show master:src/walk.rs` to see original WorkerResult enum (2 variants only) and the inlined search logic in spawn_senders.

  **API References**:
  - `src/filesystem.rs` — `make_absolute(path: &Path, cwd: &Path) -> PathBuf` (created in Task 2)
  - `src/config.rs` — `Config.cwd: Option<PathBuf>` (created in Task 1). Access as `config.cwd.as_deref()` to get `Option<&Path>`.

  **Acceptance Criteria**:
  ```bash
  # Must compile cleanly:
  cargo check 2>&1 | grep "^error"
  # Expected: no errors

  # Verify FatalError is completely gone:
  grep -r "FatalError" src/
  # Expected: no output (zero matches)

  # Verify path_absolute_form is no longer called from walk.rs:
  grep "path_absolute_form" src/walk.rs
  # Expected: no output
  ```

  **Commit**: NO (groups with Task 7)

---

- [ ] 4. Delete old test + verify CHANGELOG

  **What to do**:

  **Step 4a — Delete walk.rs test module (lines 714-738)**: Remove the entire `#[cfg(test)] mod tests` block from walk.rs. The old test (`full_path_search_returns_error_for_invalid_cwd`) tested that `search_str_for_entry` returns `Err` when cwd is invalid. With the new architecture, `search_str_for_entry` is infallible — the failure happens at startup in `construct_config`. The behavior is validated by:
  - Unit tests for `make_absolute` (Task 2)
  - Integration tests in `tests/tests.rs` (existing, test `--full-path` end-to-end)
  - The early failure path in `construct_config` (standard `anyhow` error propagation)

  **Step 4b — Verify CHANGELOG**: The current entry reads:
  > Handle invalid working directories gracefully when using `--full-path`, see #1900 (@Xavrir).
  
  This wording is still accurate — the fix still handles invalid working directories gracefully (just via early failure instead of per-entry error handling). No change needed unless the executor feels the wording should reflect "fail early" more explicitly.

  **Must NOT do**:
  - Do not delete or modify the filesystem.rs tests (those are the new make_absolute tests from Task 2)
  - Do not modify integration tests in tests/tests.rs

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Task 5)
  - **Blocks**: None
  - **Blocked By**: Task 3

  **References**:

  **File References**:
  - `src/walk.rs:714-738` — The test module to delete entirely
  - `CHANGELOG.md:8` — The changelog entry to verify (no change expected)

  **Acceptance Criteria**:
  ```bash
  # Verify test module is gone:
  grep -n "mod tests" src/walk.rs
  # Expected: no output

  # Verify CHANGELOG entry still exists:
  grep "1900" CHANGELOG.md
  # Expected: shows the bugfix line
  ```

  **Commit**: NO (groups with Task 7)

---

- [ ] 5. Build verification: cargo build + test + clippy

  **What to do**:

  Run all three verification commands. Fix any issues that arise.

  ```bash
  cargo build 2>&1
  cargo test 2>&1
  cargo clippy 2>&1
  ```

  **Common issues to watch for**:
  - Unused imports after removing FatalError (clippy will catch this)
  - Dead code warnings if any helper was only used by FatalError path
  - Test failures in integration tests if `--full-path` behavior changed unexpectedly

  **Must NOT do**:
  - Do not suppress warnings with `#[allow(...)]`
  - Do not skip clippy

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Task 4)
  - **Blocks**: Task 7
  - **Blocked By**: Task 3

  **References**:

  **Build References**:
  - Root `AGENTS.md` doesn't list fd-specific build commands, but standard Rust: `cargo build`, `cargo test`, `cargo clippy`
  - `Cargo.toml` at `/home/xavrir/fd/Cargo.toml` for dependency info

  **Acceptance Criteria**:
  ```bash
  cargo build 2>&1 | tail -1
  # Expected: "Finished" line with no errors

  cargo test 2>&1 | grep -E "^test result:"
  # Expected: "test result: ok." with 0 failures

  cargo clippy 2>&1 | grep "^warning\|^error" | grep -v "Compiling\|Finished"
  # Expected: no warnings or errors (or only pre-existing ones unrelated to our changes)
  ```

  **Commit**: NO (groups with Task 7)

---

- [ ] 6. Reply to all review comments on GitHub

  **What to do**:

  Reply to each of the 6 review comments using `gh api`. Tone: direct, concise, human. No AI slop.

  **Comment replies** (adapt exact wording, but keep this tone):

  **6a — Top-level review** (re-architecture suggestion):
  Reply via `gh api repos/sharkdp/fd/pulls/1917/reviews/{review_id}/comments` or as a regular comment:
  ```
  Good call — caching cwd once at startup is clearly better. Avoids per-entry syscalls and eliminates the need for FatalError entirely. I've reworked the PR around this approach: Config gets a `cwd: Option<PathBuf>` field populated when `--full-path` is set, and `search_str_for_entry` uses it infallibly. If cwd can't be retrieved, fd fails early before the walk starts.
  ```

  **6b — src/exec/job.rs:57** (Vec allocation in batch):
  ```
  Moot now — FatalError is gone, so batch() reverts to the original filter_map directly into execute_batch. No Vec allocation.
  ```

  **6c — src/walk.rs:238** (duplicated error handling):
  ```
  Gone — no FatalError variant means no duplicated handling to extract.
  ```

  **6d — src/walk.rs:234** (fatal errors should always print):
  ```
  Agreed, but also moot now. The cwd failure is caught at startup in construct_config, so it surfaces as a top-level error before the walk even begins.
  ```

  **6e — src/walk.rs:542** (unnecessary match on tx.send):
  ```
  Gone with the re-architecture — no FatalError to send.
  ```

  **6f — src/walk.rs:729** (test uses env::set_current_dir):
  ```
  Replaced the test entirely. The old test validated per-entry error handling which no longer exists. New unit tests for the make_absolute function don't need any process-global state.
  ```

  **Implementation**: Use `gh api` to reply to each inline comment. First fetch the comment IDs:
  ```bash
  # Get inline comment IDs
  gh api repos/sharkdp/fd/pulls/1917/comments --jq '.[] | {id, path, line, body}'
  
  # Reply to each inline comment
  gh api repos/sharkdp/fd/pulls/1917/comments/{comment_id}/replies \
    -f body="reply text"
  
  # For top-level review, post a PR comment
  gh pr comment 1917 --repo sharkdp/fd --body "reply text"
  ```

  **Must NOT do**:
  - No "Great suggestion!", "I appreciate", "Absolutely!", "This is a fantastic idea"
  - No "I've gone ahead and...", "I took the liberty of..."
  - No bullet-point lists of "changes made" — keep it conversational
  - No emoji

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Task 7, but 7 waits for Task 5)
  - **Blocks**: None
  - **Blocked By**: None (content is known upfront, can run anytime)

  **References**:

  **GitHub API References**:
  - PR #1917 review comments: `gh api repos/sharkdp/fd/pulls/1917/comments`
  - Reply to inline comment: `gh api repos/sharkdp/fd/pulls/1917/comments/{id}/replies -f body="..."`
  - Post PR-level comment: `gh pr comment 1917 --repo sharkdp/fd --body "..."`

  **Acceptance Criteria**:
  ```bash
  # Verify all replies posted (6 inline + 1 top-level = 6 total, since top-level is a PR comment)
  gh api repos/sharkdp/fd/pulls/1917/comments --jq '[.[] | select(.user.login == "Xavrir")] | length'
  # Expected: replies visible on each comment thread
  ```

  **Commit**: N/A (GitHub API, not code)

---

- [ ] 7. Commit changes + push to PR branch

  **What to do**:

  **Step 7a — Stage all changed files**:
  ```bash
  git add src/config.rs src/main.rs src/filesystem.rs src/walk.rs src/exec/job.rs
  ```
  (CHANGELOG.md only if wording was changed)

  **Step 7b — Commit** with a descriptive message:
  ```
  refactor: cache cwd at startup instead of per-entry resolution

  Instead of calling current_dir() for every entry when --full-path is
  set, cache it once in Config at startup. This eliminates the need for
  FatalError handling and makes path resolution infallible during the
  walk. If the cwd can't be retrieved, fd now fails early with a clear
  error message.
  ```

  **Step 7c — Push**:
  ```bash
  git push origin issue-1900
  ```

  **Must NOT do**:
  - Do not force push unless the push fails due to diverged history
  - Do not amend existing commits
  - Do not commit `.sisyphus/` directory files

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: [`git-master`]
    - `git-master`: For clean commit workflow

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 4 (after Task 5 confirms clean build)
  - **Blocks**: None
  - **Blocked By**: Task 5

  **References**:

  **Git References**:
  - Current branch: `issue-1900` tracking `origin/issue-1900`
  - Remote: `origin` → `git@github.com:Xavrir/fd.git`
  - Last commit: `f3bc332 ci: retrigger workflow for second validation pass`

  **Acceptance Criteria**:
  ```bash
  git status
  # Expected: clean working tree, branch ahead of origin

  git log --oneline -3
  # Expected: new commit at HEAD with the refactor message

  git push origin issue-1900
  # Expected: push succeeds
  ```

  **Commit**: YES
  - Message: `refactor: cache cwd at startup instead of per-entry resolution`
  - Files: `src/config.rs`, `src/main.rs`, `src/filesystem.rs`, `src/walk.rs`, `src/exec/job.rs`
  - Pre-commit: `cargo test`

---

## Commit Strategy

| After Task | Message | Files | Verification |
|------------|---------|-------|--------------|
| 7 (all tasks) | `refactor: cache cwd at startup instead of per-entry resolution` | config.rs, main.rs, filesystem.rs, walk.rs, exec/job.rs | cargo build + test + clippy |

Single atomic commit for the entire re-architecture — all changes are tightly coupled and should land together.

---

## Success Criteria

### Verification Commands
```bash
cargo build          # Expected: compiles cleanly
cargo test           # Expected: all tests pass
cargo clippy         # Expected: no warnings
grep -r "FatalError" src/  # Expected: zero matches
grep "make_absolute" src/filesystem.rs  # Expected: function exists
grep "cwd" src/config.rs  # Expected: field exists
```

### Final Checklist
- [ ] `cwd: Option<PathBuf>` exists in Config
- [ ] `construct_config` fails early when `--full-path` and cwd is invalid
- [ ] `make_absolute` function exists with unit tests
- [ ] `search_str_for_entry` is infallible (no `Result` in signature)
- [ ] No `FatalError` variant anywhere in codebase
- [ ] `batch()` uses `filter_map` (no Vec)
- [ ] Old walk.rs test deleted
- [ ] All 6 review comments replied to
- [ ] Changes pushed to origin/issue-1900
- [ ] `cargo build` + `cargo test` + `cargo clippy` all clean
