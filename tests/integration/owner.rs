#[cfg(unix)]
use nix::unistd::{Gid, Group, Uid, User};

use crate::testenv::{TestEnv, DEFAULT_DIRS, DEFAULT_FILES};

#[cfg(unix)]
#[test]
fn test_owner_ignore_all() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);
    te.assert_output(&["--owner", ":", "a.foo"], "a.foo");
    te.assert_output(&["--owner", "", "a.foo"], "a.foo");
}

#[cfg(unix)]
#[test]
fn test_owner_current_user() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);
    let uid = Uid::current();
    te.assert_output(&["--owner", &uid.to_string(), "a.foo"], "a.foo");
    if let Ok(Some(user)) = User::from_uid(uid) {
        te.assert_output(&["--owner", &user.name, "a.foo"], "a.foo");
    }
}

#[cfg(unix)]
#[test]
fn test_owner_current_group() {
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);
    let gid = Gid::current();
    te.assert_output(&["--owner", &format!(":{gid}"), "a.foo"], "a.foo");
    if let Ok(Some(group)) = Group::from_gid(gid) {
        te.assert_output(&["--owner", &format!(":{}", group.name), "a.foo"], "a.foo");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_owner_root() {
    // This test assumes the current user isn't root
    if Uid::current().is_root() || Gid::current() == Gid::from_raw(0) {
        return;
    }
    let te = TestEnv::new(DEFAULT_DIRS, DEFAULT_FILES);
    te.assert_output(&["--owner", "root", "a.foo"], "");
    te.assert_output(&["--owner", "0", "a.foo"], "");
    te.assert_output(&["--owner", ":root", "a.foo"], "");
    te.assert_output(&["--owner", ":0", "a.foo"], "");
}
