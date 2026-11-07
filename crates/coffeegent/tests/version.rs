//! CLI smoke tests for M0.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn version_flag_prints_workspace_version() {
    Command::cargo_bin("coffeegent")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"));
}

#[test]
fn default_invocation_prints_version() {
    Command::cargo_bin("coffeegent")
        .unwrap()
        .assert()
        .success()
        .stdout(predicate::str::contains("coffeegent 0.1.0"));
}

#[test]
fn hook_always_exits_zero() {
    Command::cargo_bin("coffeegent")
        .unwrap()
        .args(["hook", "claude"])
        .assert()
        .code(0)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}
