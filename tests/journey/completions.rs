//! `pace completions SHELL` prints the completion script for the shell.

use rstest::rstest;

use crate::support::Sandbox;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[rstest]
#[case::bash("bash", "complete -F _pace")]
#[case::zsh("zsh", "#compdef pace")]
#[case::fish("fish", "complete -c pace")]
#[case::powershell("powershell", "Register-ArgumentCompleter")]
#[case::elvish("elvish", "edit:completion:arg-completer[pace]")]
#[case::nushell("nushell", "export extern pace")]
fn completions_cover_every_shell(#[case] shell: &str, #[case] marker: &str) -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.pace().args(["completions", shell]).output()?;

    assert!(output.status.success(), "{output:?}");
    let script = String::from_utf8(output.stdout)?;
    assert!(script.contains(marker), "{script}");
    for command in ["begin", "adjust", "forget", "store", "completions"] {
        assert!(script.contains(command), "{shell} lacks {command}");
    }
    Ok(())
}

#[test]
fn an_unknown_shell_is_a_usage_error() -> TestResult {
    let sandbox = Sandbox::new()?;

    let output = sandbox.pace().args(["completions", "cmd"]).output()?;

    assert_eq!(output.status.code(), Some(2));
    Ok(())
}
