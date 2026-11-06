//! Runs the script containers of the official image run, with `bash`, in a
//! directory where `config.sh` and `run.sh` are stand-ins that write down
//! how they were called: what the script does on a container's first start
//! and on the starts after it, without GitHub or a container.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

use swarm_application::ports::{LaunchSpec, RunnerImage};
use swarm_domain::{
    Forge, RegistrationToken, RunnerDefinition, RunnerId, RunnerLabels, RunnerLifecycle,
    RunnerName, RunnerScope,
};
use swarm_github::GitHubOfficialImage;
use tempfile::TempDir;

const TOKEN: &str = "AESOHC2SEXAMPLE";

fn spec(labels: &str, lifecycle: RunnerLifecycle) -> LaunchSpec {
    let runner = RunnerDefinition::new(
        RunnerId::generate(),
        Forge::GitHub,
        RunnerScope::parse_url("https://github.com/otsobide/mindweaver").unwrap(),
        RunnerName::new("builder").unwrap(),
        RunnerLabels::parse_comma_separated(labels).unwrap(),
        lifecycle,
    );
    GitHubOfficialImage::default().launch_spec(&runner, &RegistrationToken::new(TOKEN).unwrap())
}

/// A runner's directory, with stand-ins for its scripts: `config.sh`
/// writes its arguments to `config.args` and registers (`.runner`), or
/// fails if `config_fails`; `run.sh` writes its environment to `run.env`.
fn runner_directory(config_fails: bool) -> TempDir {
    let directory = tempfile::tempdir().unwrap();
    let config = if config_fails {
        "echo 'Http response code: NotFound' >&2; exit 1"
    } else {
        "printf '%s\\n' \"$@\" > config.args; touch .runner"
    };
    write_script(directory.path(), "config.sh", config);
    write_script(directory.path(), "run.sh", "env > run.env");
    directory
}

fn write_script(directory: &Path, name: &str, body: &str) {
    let path = directory.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// Runs the spec's command in `directory`, with the spec's environment and
/// nothing else but `PATH`, as a container would.
fn start(directory: &Path, spec: &LaunchSpec) -> Output {
    Command::new(&spec.command[0])
        .args(&spec.command[1..])
        .current_dir(directory)
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap())
        .envs(
            spec.environment
                .iter()
                .map(|variable| (variable.name(), variable.value())),
        )
        .output()
        .unwrap()
}

fn config_args(directory: &Path) -> Vec<String> {
    fs::read_to_string(directory.join("config.args"))
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The value following `flag` in `args`.
fn after<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|at| args.get(at + 1))
        .map(String::as_str)
}

#[test]
fn registers_the_runner_on_its_first_start_then_runs_it() {
    let directory = runner_directory(false);

    let output = start(
        directory.path(),
        &spec("gpu,large", RunnerLifecycle::Persistent),
    );

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let args = config_args(directory.path());
    assert!(args.contains(&"--unattended".to_owned()));
    assert!(args.contains(&"--replace".to_owned()));
    assert_eq!(
        after(&args, "--url"),
        Some("https://github.com/otsobide/mindweaver")
    );
    assert_eq!(after(&args, "--token"), Some(TOKEN));
    assert_eq!(after(&args, "--name"), Some("builder"));
    assert_eq!(after(&args, "--labels"), Some("gpu,large"));
    assert!(!args.contains(&"--ephemeral".to_owned()));
    assert!(directory.path().join("run.env").exists());
}

#[test]
fn registers_an_ephemeral_runner_as_one() {
    let directory = runner_directory(false);

    start(directory.path(), &spec("", RunnerLifecycle::Ephemeral));

    assert!(config_args(directory.path()).contains(&"--ephemeral".to_owned()));
}

#[test]
fn passes_no_labels_when_the_runner_has_none() {
    let directory = runner_directory(false);

    start(directory.path(), &spec("", RunnerLifecycle::Persistent));

    assert!(!config_args(directory.path()).contains(&"--labels".to_owned()));
}

#[test]
fn does_not_register_again_when_started_again() {
    let directory = runner_directory(false);
    fs::write(directory.path().join(".runner"), "{}").unwrap();

    let output = start(directory.path(), &spec("", RunnerLifecycle::Persistent));

    assert!(output.status.success());
    assert!(!directory.path().join("config.args").exists());
    assert!(directory.path().join("run.env").exists());
}

#[test]
fn runs_the_runner_without_the_token_in_its_environment() {
    let directory = runner_directory(false);

    start(directory.path(), &spec("", RunnerLifecycle::Persistent));

    let environment = fs::read_to_string(directory.path().join("run.env")).unwrap();
    assert!(environment.contains("RUNNER_NAME=builder"));
    assert!(!environment.contains("RUNNER_TOKEN"));
    assert!(!environment.contains(TOKEN));
}

#[test]
fn fails_without_running_the_runner_when_it_cannot_register() {
    let directory = runner_directory(true);

    let output = start(directory.path(), &spec("", RunnerLifecycle::Persistent));

    assert!(!output.status.success());
    assert!(!directory.path().join("run.env").exists());
}
