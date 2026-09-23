mod testenv;

use crate::testenv::TestEnv;

/// --min-depth should not silently drop broken symlinks when following links.
/// Regression test for https://github.com/sharkdp/fd/issues/1017.
#[test]
fn test_min_depth_broken_symlink() {
    let mut te = TestEnv::new(&["one/two"], &["one/two/file.txt"]);
    te.create_broken_symlink("broken_symlink")
        .expect("Failed to create broken symlink.");

    // Broken symlinks have no known depth, so --min-depth must not filter them out.
    // (Without the fix, the broken symlink is silently dropped.)
    te.assert_output(&["--follow", "--min-depth", "2", "symlink"], "broken_symlink");

    // Sanity check: a valid symlink at depth 1 is still filtered by --min-depth 2,
    // and both are listed with --min-depth 1.
    te.assert_output(
        &["--follow", "--min-depth", "1", "symlink"],
        "broken_symlink\nsymlink",
    );
}
