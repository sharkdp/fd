mod testenv;

use crate::testenv::{get_test_env_with_abs_path, TestEnv, DEFAULT_DIRS, DEFAULT_FILES};

/// Shell script execution (--exec)
#[test]
fn test_exec() {
    let (te, abs_path) = get_test_env_with_abs_path(DEFAULT_DIRS, DEFAULT_FILES);
    // TODO Windows tests: D:file.txt \file.txt \\server\share\file.txt ...
    if !cfg!(windows) {
        te.assert_output(
            &["--absolute-path", "foo", "--exec", "echo"],
            &format!(
                "{abs_path}/a.foo
                {abs_path}/one/b.foo
                {abs_path}/one/two/C.Foo2
                {abs_path}/one/two/c.foo
                {abs_path}/one/two/three/d.foo
                {abs_path}/one/two/three/directory_foo",
                abs_path = &abs_path
            ),
        );

        te.assert_output(
            &["foo", "--exec", "echo", "{}"],
            "./a.foo
            ./one/b.foo
            ./one/two/C.Foo2
            ./one/two/c.foo
            ./one/two/three/d.foo
            ./one/two/three/directory_foo",
        );

        te.assert_output(
            &["foo", "--strip-cwd-prefix", "--exec", "echo", "{}"],
            "a.foo
            one/b.foo
            one/two/C.Foo2
            one/two/c.foo
            one/two/three/d.foo
            one/two/three/directory_foo",
        );

        te.assert_output(
            &["foo", "--exec", "echo", "{.}"],
            "a
            one/b
            one/two/C
            one/two/c
            one/two/three/d
            one/two/three/directory_foo",
        );

        te.assert_output(
            &["foo", "--exec", "echo", "{/}"],
            "a.foo
            b.foo
            C.Foo2
            c.foo
            d.foo
            directory_foo",
        );

        te.assert_output(
            &["foo", "--exec", "echo", "{/.}"],
            "a
            b
            C
            c
            d
            directory_foo",
        );

        te.assert_output(
            &["foo", "--exec", "echo", "{//}"],
            ".
            ./one
            ./one/two
            ./one/two
            ./one/two/three
            ./one/two/three",
        );

        te.assert_output(&["e1", "--exec", "printf", "%s.%s\n"], "./e1 e2.");
    }
}

// TODO test for windows
#[cfg(not(windows))]
#[test]
fn test_exec_multi() {
    let (te, abs_path) = get_test_env_with_abs_path(DEFAULT_DIRS, DEFAULT_FILES);

    te.assert_output(
        &[
            "--absolute-path",
            "foo",
            "--exec",
            "echo",
            ";",
            "--exec",
            "echo",
            "test",
            "{/}",
        ],
        &format!(
            "{abs_path}/a.foo
                {abs_path}/one/b.foo
                {abs_path}/one/two/C.Foo2
                {abs_path}/one/two/c.foo
                {abs_path}/one/two/three/d.foo
                {abs_path}/one/two/three/directory_foo
                test a.foo
                test b.foo
                test C.Foo2
                test c.foo
                test d.foo
                test directory_foo",
            abs_path = &abs_path
        ),
    );

    te.assert_output(
        &[
            "e1", "--exec", "echo", "{.}", ";", "--exec", "echo", "{/}", ";", "--exec", "echo",
            "{//}", ";", "--exec", "echo", "{/.}",
        ],
        "e1 e2
        e1 e2
        .
        e1 e2",
    );

    // We use printf here because we need to suppress a newline and
    // echo -n is not POSIX-compliant.
    te.assert_output(
        &[
            "foo", "--exec", "printf", "%s", "{/}: ", ";", "--exec", "printf", "%s\\n", "{//}",
        ],
        "a.foo: .
        b.foo: ./one
        C.Foo2: ./one/two
        c.foo: ./one/two
        d.foo: ./one/two/three
        directory_foo: ./one/two/three",
    );
}

#[test]
fn test_exec_batch() {
    let (te, abs_path) = get_test_env_with_abs_path(DEFAULT_DIRS, DEFAULT_FILES);
    let te = te.normalize_line(true);

    // TODO Test for windows
    if !cfg!(windows) {
        te.assert_output(
            &["--absolute-path", "foo", "--exec-batch", "echo"],
            &format!(
                "{abs_path}/a.foo {abs_path}/one/b.foo {abs_path}/one/two/C.Foo2 {abs_path}/one/two/c.foo {abs_path}/one/two/three/d.foo {abs_path}/one/two/three/directory_foo",
                abs_path = &abs_path
            ),
        );

        te.assert_output(
            &["foo", "--exec-batch", "echo", "{}"],
            "./a.foo ./one/b.foo ./one/two/C.Foo2 ./one/two/c.foo ./one/two/three/d.foo ./one/two/three/directory_foo",
        );

        te.assert_output(
            &["foo", "--strip-cwd-prefix", "--exec-batch", "echo", "{}"],
            "a.foo one/b.foo one/two/C.Foo2 one/two/c.foo one/two/three/d.foo one/two/three/directory_foo",
        );

        te.assert_output(
            &["foo", "--exec-batch", "echo", "{/}"],
            "a.foo b.foo C.Foo2 c.foo d.foo directory_foo",
        );

        te.assert_output(
            &["no_match", "--exec-batch", "echo", "Matched: ", "{/}"],
            "",
        );

        te.assert_failure_with_error(
            &["foo", "--exec-batch", "echo", "{}", "{}"],
            "error: Only one placeholder allowed for batch commands\n\
            \n\
            Usage: fd [OPTIONS] [pattern] [path]...\n\
            \n\
            For more information, try '--help'.\n\
            ",
        );

        te.assert_failure_with_error(
            &["foo", "--exec-batch", "echo", "{/}", ";", "-x", "echo"],
            "error: the argument '--exec-batch <cmd>...' cannot be used with '--exec <cmd>...'\n\
            \n\
            Usage: fd --exec-batch <cmd>... <pattern> [path]...\n\
            \n\
            For more information, try '--help'.\n\
            ",
        );

        te.assert_failure_with_error(
            &["foo", "--exec-batch"],
            "error: a value is required for '--exec-batch <cmd>...' but none was supplied\n\
            \n\
            For more information, try '--help'.\n\
            ",
        );

        te.assert_failure_with_error(
            &["foo", "--exec-batch", "echo {}"],
            "error: First argument of exec-batch is expected to be a fixed executable\n\
            \n\
            Usage: fd [OPTIONS] [pattern] [path]...\n\
            \n\
            For more information, try '--help'.\n\
            ",
        );

        te.assert_failure_with_error(&["a.foo", "--exec-batch", "bash", "-c", "exit 1"], "");
    }
}

#[test]
fn test_exec_batch_multi() {
    // TODO test for windows
    if cfg!(windows) {
        return;
    }
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    let output = te.assert_success_and_get_output(
        ".",
        &[
            "foo",
            "--exec-batch",
            "echo",
            "{}",
            ";",
            "--exec-batch",
            "echo",
            "{/}",
        ],
    );
    let stdout = std::str::from_utf8(&output.stdout).unwrap();
    let lines: Vec<_> = stdout
        .lines()
        .map(|l| {
            let mut words: Vec<_> = l.split_whitespace().collect();
            words.sort_unstable();
            words
        })
        .collect();

    assert_eq!(
        lines,
        &[
            [
                "./a.foo",
                "./one/b.foo",
                "./one/two/C.Foo2",
                "./one/two/c.foo",
                "./one/two/three/d.foo",
                "./one/two/three/directory_foo"
            ],
            [
                "C.Foo2",
                "a.foo",
                "b.foo",
                "c.foo",
                "d.foo",
                "directory_foo"
            ],
        ]
    );

    te.assert_failure_with_error(
        &[
            "a.foo",
            "--exec-batch",
            "echo",
            ";",
            "--exec-batch",
            "bash",
            "-c",
            "exit 1",
        ],
        "",
    );
}

#[test]
fn test_exec_batch_with_limit() {
    // TODO Test for windows
    if cfg!(windows) {
        return;
    }

    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);

    let output = te.assert_success_and_get_output(
        ".",
        &["foo", "--batch-size=2", "--exec-batch", "echo", "{}"],
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        assert_eq!(2, line.split_whitespace().count());
    }

    let mut paths: Vec<_> = stdout
        .lines()
        .flat_map(|line| line.split_whitespace())
        .collect();
    paths.sort_unstable();
    assert_eq!(
        &paths,
        &[
            "./a.foo",
            "./one/b.foo",
            "./one/two/C.Foo2",
            "./one/two/c.foo",
            "./one/two/three/d.foo",
            "./one/two/three/directory_foo"
        ],
    );
}

/// Shell script execution (--exec) with a custom --path-separator
#[test]
fn test_exec_with_separator() {
    let (te, abs_path) = get_test_env_with_abs_path(DEFAULT_DIRS, DEFAULT_FILES);
    te.assert_output(
        &[
            "--path-separator=#",
            "--absolute-path",
            "foo",
            "--exec",
            "echo",
        ],
        &format!(
            "{abs_path}#a.foo
                {abs_path}#one#b.foo
                {abs_path}#one#two#C.Foo2
                {abs_path}#one#two#c.foo
                {abs_path}#one#two#three#d.foo
                {abs_path}#one#two#three#directory_foo",
            abs_path = abs_path.replace(std::path::MAIN_SEPARATOR, "#"),
        ),
    );

    te.assert_output(
        &["--path-separator=#", "foo", "--exec", "echo", "{}"],
        ".#a.foo
            .#one#b.foo
            .#one#two#C.Foo2
            .#one#two#c.foo
            .#one#two#three#d.foo
            .#one#two#three#directory_foo",
    );

    te.assert_output(
        &["--path-separator=#", "foo", "--exec", "echo", "{.}"],
        "a
            one#b
            one#two#C
            one#two#c
            one#two#three#d
            one#two#three#directory_foo",
    );

    te.assert_output(
        &["--path-separator=#", "foo", "--exec", "echo", "{/}"],
        "a.foo
            b.foo
            C.Foo2
            c.foo
            d.foo
            directory_foo",
    );

    te.assert_output(
        &["--path-separator=#", "foo", "--exec", "echo", "{/.}"],
        "a
            b
            C
            c
            d
            directory_foo",
    );

    te.assert_output(
        &["--path-separator=#", "foo", "--exec", "echo", "{//}"],
        ".
            .#one
            .#one#two
            .#one#two
            .#one#two#three
            .#one#two#three",
    );

    te.assert_output(
        &["--path-separator=#", "e1", "--exec", "printf", "%s.%s\n"],
        ".#e1 e2.",
    );
}
