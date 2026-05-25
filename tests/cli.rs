// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Fabian Schmieder

//! CLI integration tests — verify subcommands parse and execute correctly.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help() {
    Command::cargo_bin("devserial")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: devserial"))
        .stdout(predicate::str::contains("mcp"))
        .stdout(predicate::str::contains("list"));
}

#[test]
fn test_version() {
    Command::cargo_bin("devserial")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("devserial"));
}

#[test]
fn test_list_command() {
    Command::cargo_bin("devserial")
        .unwrap()
        .arg("list")
        .assert()
        .success();
}

#[test]
#[cfg(feature = "monitor")]
fn test_monitor_help() {
    Command::cargo_bin("devserial")
        .unwrap()
        .args(["monitor", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Serial port path"));
}

#[test]
#[cfg(feature = "tui")]
fn test_tui_help() {
    Command::cargo_bin("devserial")
        .unwrap()
        .args(["tui", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Serial port path"));
}

#[test]
fn test_flash_help() {
    Command::cargo_bin("devserial")
        .unwrap()
        .args(["flash", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("firmware"));
}
