//! `pace completions SHELL` prints the completion script for the shell.

use rstest::rstest;

use crate::support::Sandbox;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Every command, as its path below `pace`.
const COMMANDS: [&[&str]; 39] = [
    &["begin"],
    &["hold"],
    &["pause"],
    &["resume"],
    &["catch"],
    &["end"],
    &["now"],
    &["adjust"],
    &["log"],
    &["edit"],
    &["undo"],
    &["forget"],
    &["reflect"],
    &["bill"],
    &["export"],
    &["role"],
    &["role", "add"],
    &["role", "list"],
    &["role", "archive"],
    &["role", "edit"],
    &["client"],
    &["client", "add"],
    &["client", "list"],
    &["client", "edit"],
    &["project"],
    &["project", "init"],
    &["project", "add"],
    &["project", "list"],
    &["project", "archive"],
    &["project", "edit"],
    &["setup"],
    &["config"],
    &["config", "show"],
    &["config", "get"],
    &["config", "set"],
    &["doctor"],
    &["store"],
    &["store", "split"],
    &["completions"],
];

/// The flags that phase 3 added to the commands.
const FLAGS: [&str; 22] = [
    "role",
    "by-me",
    "by-others",
    "billable",
    "not-billable",
    "allow-overlap",
    "change-billed",
    "client",
    "device",
    "format",
    "template",
    "compare",
    "month",
    "mark-billed",
    "invoice",
    "done-when",
    "json",
    "start-over",
    "visible",
    "weekly-target",
    "rounding-step",
    "budget",
];

/// The text by which each shell's script completes the command `path`.
fn bash(path: &[&str]) -> String {
    format!("cmd=\"pace__subcmd__{}\"", path.join("__subcmd__"))
}

fn zsh(path: &[&str]) -> String {
    format!("'{}:", path.last().unwrap_or(&""))
}

fn fish(path: &[&str]) -> String {
    format!("-a \"{}\"", path.last().unwrap_or(&""))
}

fn powershell(path: &[&str]) -> String {
    format!("'pace;{}'", path.join(";"))
}

fn elvish(path: &[&str]) -> String {
    format!("&'pace;{}'=", path.join(";"))
}

fn nushell(path: &[&str]) -> String {
    format!("export extern \"pace {}\"", path.join(" "))
}

#[rstest]
#[case::bash("bash", "complete -F _pace", bash)]
#[case::zsh("zsh", "#compdef pace", zsh)]
#[case::fish("fish", "complete -c pace", fish)]
#[case::powershell("powershell", "Register-ArgumentCompleter", powershell)]
#[case::elvish("elvish", "edit:completion:arg-completer[pace]", elvish)]
#[case::nushell("nushell", "export extern pace", nushell)]
fn completions_cover_every_shell_and_command(
    #[case] shell: &str,
    #[case] marker: &str,
    #[case] completes: fn(&[&str]) -> String,
) -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.pace().args(["completions", shell]).output()?;

    assert!(output.status.success(), "{output:?}");
    let script = String::from_utf8(output.stdout)?;
    assert!(script.contains(marker), "{script}");
    for path in COMMANDS {
        let expected = completes(path);
        assert!(
            script.contains(&expected),
            "{shell} lacks `{}`: no {expected}",
            path.join(" ")
        );
    }
    Ok(())
}

/// The text by which each shell's script completes the flag `--name`.
fn fish_flag(name: &str) -> String {
    format!("-l {name}")
}

fn long_flag(name: &str) -> String {
    format!("--{name}")
}

#[rstest]
#[case::bash("bash", long_flag)]
#[case::zsh("zsh", long_flag)]
#[case::fish("fish", fish_flag)]
#[case::powershell("powershell", long_flag)]
#[case::elvish("elvish", long_flag)]
#[case::nushell("nushell", long_flag)]
fn completions_cover_flags_of_phase_3(
    #[case] shell: &str,
    #[case] completes: fn(&str) -> String,
) -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.pace().args(["completions", shell]).output()?;

    assert!(output.status.success(), "{output:?}");
    let script = String::from_utf8(output.stdout)?;
    for flag in FLAGS {
        let expected = completes(flag);
        assert!(
            script.contains(&expected),
            "{shell} lacks `--{flag}`: no {expected}"
        );
    }
    Ok(())
}

#[test]
fn an_unknown_shell_is_usage_error() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.pace().args(["completions", "cmd"]).output()?;

    assert_eq!(output.status.code(), Some(2));
    Ok(())
}

#[test]
fn completions_into_closed_pipe_end_quietly() -> TestResult {
    let sandbox = Sandbox::new()?;
    let (reader, writer) = std::io::pipe()?;
    drop(reader);

    let output = sandbox
        .pace()
        .args(["completions", "zsh"])
        .stdout(writer)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(String::from_utf8(output.stderr)?, "");
    Ok(())
}
