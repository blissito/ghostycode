//! The kill switch has to reach the in-process runtime that would emit.
//!
//! The single `ghosty` binary resolves dispatcher overrides, states the
//! telemetry floor in its environment, and then calls `ghosty_tui::run`.
//! That runtime re-resolves telemetry before it can arm. These tests drive the
//! real binary through the keyless `features list` command and use the local
//! dry-run sink as the end-to-end observable: an enabled positive control must
//! write session events, while a kill switch must create no telemetry state.

#![cfg(unix)]

use std::fs;
use std::process::Command;

use ghosty_config::{SetupState, TELEMETRY_NOTICE_VERSION};
use tempfile::TempDir;

/// `GHOSTY_TELEMETRY=0` beats `--telemetry true`, end to end.
///
/// The positive control proves the runtime is enabled before the kill switch is
/// applied, so the zero-state assertion cannot pass vacuously.
#[test]
fn env_off_beats_cli_on_end_to_end() {
    // Positive control first: the flag reaches the in-process runtime and
    // arms its dry-run sink, so the assertion below is about the floor and not
    // about a command that never crossed the dispatch boundary.
    let on = dispatch_and_read_telemetry(None);
    let dry_run = on
        .dry_run
        .expect("`--telemetry true` must write the dry-run sink");
    assert!(
        dry_run.contains("\"event\":\"session_start\"")
            && dry_run.contains("\"event\":\"session_end\""),
        "the real in-process runtime must record a complete session: {dry_run}"
    );

    let off = dispatch_and_read_telemetry(Some("0"));
    assert!(
        !off.telemetry_dir_exists && off.dry_run.is_none(),
        "`GHOSTY_TELEMETRY=0` must beat `--telemetry true` before the runtime arms"
    );
}

/// A real endpoint must remain queued locally instead of becoming short-CLI
/// network latency. The next interactive session owns delivery.
#[test]
fn short_cli_exit_persists_without_network_delivery() {
    let evidence = dispatch_and_read_telemetry_with_endpoint(None, None);
    let pending = evidence
        .pending
        .expect("short CLI must seal its pending telemetry before process exit");
    assert!(
        pending.contains("\"event\":\"session_start\"")
            && pending.contains("\"event\":\"session_end\""),
        "the pending buffer must contain the complete short CLI session: {pending}"
    );
    assert!(
        evidence.dry_run.is_none(),
        "a configured endpoint must stay pending rather than use the dry-run sink"
    );
}

/// A value the resolver cannot parse resolves to off, rather than falling
/// through to the flag.
#[test]
fn an_unparseable_telemetry_env_value_keeps_the_in_process_runtime_off() {
    let evidence = dispatch_and_read_telemetry(Some("maybe"));
    assert!(
        !evidence.telemetry_dir_exists && evidence.dry_run.is_none(),
        "a typo in the kill switch must never arm the in-process runtime"
    );
}

/// Re-enabling through the documented settings command must clear a decline
/// recorded by the former opt-in notice as well as the config-file floor.
#[test]
fn config_set_true_reenables_a_historical_decline() {
    let fixture = TempDir::new().expect("fixture root");
    let home = fixture.path().join("home");
    let ghosty_home = fixture.path().join("ghosty-home");
    let workspace = fixture.path().join("workspace");
    for dir in [&home, &ghosty_home, &workspace] {
        fs::create_dir_all(dir).expect("create fixture dir");
    }

    let mut state = SetupState::default();
    state.record_telemetry_notice("1", false);
    let state_path = ghosty_home.join("setup_state.json");
    state
        .save_to(&state_path)
        .expect("write historical decline");

    let config_path = fixture.path().join("config.toml");
    fs::write(&config_path, "telemetry = false\n").expect("write config");

    let output = Command::new(ghosty_binary())
        .current_dir(&workspace)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").expect("PATH"))
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("GHOSTY_HOME", &ghosty_home)
        .env("GHOSTY_SECRET_BACKEND", "file")
        .args([
            "--config",
            config_path.to_str().expect("config path"),
            "config",
            "set",
            "telemetry",
            "true",
        ])
        .output()
        .expect("run config set");
    assert!(
        output.status.success(),
        "config set failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        fs::read_to_string(&config_path)
            .expect("read config")
            .contains("telemetry = true")
    );
    let state = SetupState::load_from(&state_path).expect("read setup state");
    assert!(state.telemetry_accepted(TELEMETRY_NOTICE_VERSION));
    assert!(!state.telemetry_opted_out());
}

struct DispatchEvidence {
    telemetry_dir_exists: bool,
    dry_run: Option<String>,
    pending: Option<String>,
}

/// Run the real dispatcher into a keyless in-process command and report the
/// telemetry state it actually left behind.
fn dispatch_and_read_telemetry(telemetry_env: Option<&str>) -> DispatchEvidence {
    dispatch_and_read_telemetry_with_endpoint(telemetry_env, Some(""))
}

fn dispatch_and_read_telemetry_with_endpoint(
    telemetry_env: Option<&str>,
    endpoint: Option<&str>,
) -> DispatchEvidence {
    let fixture = TempDir::new().expect("fixture root");
    let home = fixture.path().join("home");
    let ghosty_home = fixture.path().join("ghosty-home");
    let workspace = fixture.path().join("workspace");
    for dir in [&home, &ghosty_home, &workspace] {
        fs::create_dir_all(dir).expect("create fixture dir");
    }

    let config_path = fixture.path().join("config.toml");
    let mut config = "telemetry = true\n".to_string();
    if let Some(endpoint) = endpoint {
        config.push_str(&format!("telemetry_endpoint = {endpoint:?}\n"));
    }
    fs::write(&config_path, config).expect("write config");

    let mut command = Command::new(ghosty_binary());
    command
        .current_dir(&workspace)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").expect("PATH"))
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("GHOSTY_HOME", &ghosty_home)
        .env("GHOSTY_SECRET_BACKEND", "file")
        .env(
            "GHOSTY_RELEASE_BASE_URL",
            "https://example.invalid/releases",
        )
        .arg("--config")
        .arg(&config_path)
        .args(["--telemetry", "true", "features", "list"]);
    if let Some(value) = telemetry_env {
        command.env("GHOSTY_TELEMETRY", value);
    }
    let output = command.output().expect("run ghosty dispatcher");
    assert!(
        output.status.success(),
        "the in-process feature command must succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("feature\tstage\tenabled"),
        "the real in-process feature command must have run\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let telemetry_dir = ghosty_home.join("telemetry");
    let dry_run = match fs::read_to_string(telemetry_dir.join("dryrun.jsonl")) {
        Ok(contents) => Some(contents),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => panic!("read telemetry dry-run sink: {error}"),
    };
    let pending = match fs::read_to_string(telemetry_dir.join("buffer.jsonl")) {
        Ok(contents) => Some(contents),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => panic!("read telemetry pending buffer: {error}"),
    };
    DispatchEvidence {
        telemetry_dir_exists: telemetry_dir.exists(),
        dry_run,
        pending,
    }
}

fn ghosty_binary() -> &'static str {
    env!("CARGO_BIN_EXE_ghosty")
}
