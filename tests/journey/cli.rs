use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
use std::process::Command;

fn pace() -> Command {
    Command::new(get_cargo_bin("pace"))
}

#[test]
fn version_names_the_binary_and_its_version() {
    assert_cmd_snapshot!(pace().arg("--version"));
}

#[test]
fn help_describes_pace() {
    assert_cmd_snapshot!(pace().arg("--help"));
}

#[test]
fn pace_without_arguments_shows_the_help_as_a_usage_error() {
    assert_cmd_snapshot!(pace());
}
