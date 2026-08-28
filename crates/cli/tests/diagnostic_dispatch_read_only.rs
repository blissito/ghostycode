//! Diagnostic dispatch must be read-only whether the command uses the real
//! in-process TUI entry (`doctor`, `setup --status`) or stays in the CLI
//! (`auth status --diagnostic`). The single `ghosty` binary has no sibling
//! TUI executable to delegate to (#5259 single-binary argv0 dispatch). These
//! invariants stay: the dispatcher must not migrate legacy secrets, must not
//! rewrite legacy settings, and must not create any state under a sealed HOME.
//! `doctor --context-json` must still emit a machine-readable context source
//! map (`{"entries":[...]}`).

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ghosty_secrets::{FileKeyringStore, KeyringStore};
use tempfile::TempDir;

#[test]
fn dispatcher_diagnostics_are_in_process_and_read_only() {
    // (cli args, whether stdout must be a JSON object carrying an `entries`
    // array, whether this is the structural auth diagnostic). Only
    // `doctor --context-json` carries the context source map.
    for (args, expects_entries_json, expects_auth_diagnostic) in [
        (&["doctor"][..], false, false),
        (&["doctor", "--json"][..], false, false),
        (&["doctor", "--context-json"][..], true, false),
        (&["setup", "--status"][..], false, false),
        (&["auth", "status", "--diagnostic"][..], false, true),
    ] {
        let fixture = TempDir::new().expect("fixture root");
        let sealed_home = fixture.path().join("sealed-home");
        let ghosty_home = fixture.path().join("sealed-ghosty-home");
        let primary_home = sealed_home.join(".ghosty");
        let legacy = sealed_home
            .join(".deepseek")
            .join("secrets")
            .join("secrets.json");
        let legacy_settings = sealed_home.join(".deepseek").join("settings.toml");
        let legacy_settings_bytes = b"default_mode = \"plan\"\n";
        FileKeyringStore::new(&legacy)
            .set("deepseek", "synthetic-legacy-fixture")
            .expect("seed synthetic legacy store");
        fs::write(&legacy_settings, legacy_settings_bytes).expect("seed legacy settings");
        let before_paths = relative_paths(&sealed_home);
        let before_legacy = fs::read(&legacy).expect("read synthetic legacy store");

        // The diagnostic runs entirely in-process: the single `ghosty` binary
        // dispatches through `run_tui_in_process` -> `ghosty_tui::run`. No
        // `DEEPSEEK_TUI_BIN` sibling is spawned, so there is no receipt to read;
        // assert the in-process behavior and the read-only invariants instead.
        let mut command = Command::new(ghosty_binary());
        command
            .args(args)
            .env_clear()
            .env("HOME", &sealed_home)
            .env("USERPROFILE", &sealed_home)
            .env("GHOSTY_HOME", &ghosty_home)
            .env("GHOSTY_SECRET_BACKEND", "file");
        preserve_host_rustup_home(&mut command);
        let output = command.output().expect("run dispatcher diagnostic");

        assert!(
            output.status.success(),
            "dispatcher {args:?} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        if expects_entries_json {
            let report: serde_json::Value = serde_json::from_slice(&output.stdout)
                .unwrap_or_else(|error| {
                    panic!(
                        "doctor --context-json must emit a machine-readable context source map: {error}\nstdout:\n{}\nstderr:\n{}",
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr)
                    )
                });
            assert!(
                report["entries"].is_array(),
                "doctor --context-json must carry an `entries` array\nstdout:\n{}",
                String::from_utf8_lossy(&output.stdout)
            );
        }

        if expects_auth_diagnostic {
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                stdout.contains(
                    "auth diagnostic (structural only; credential values are never printed and provider credential stores were not opened)"
                ),
                "{stdout}"
            );
            assert!(
                stdout.contains(&format!(
                    "ghosty home: {}",
                    ghosty_config::quote_os_path(&ghosty_home)
                )),
                "{stdout}"
            );
            assert!(
                stdout.contains(&format!(
                    "config: {}",
                    ghosty_config::quote_os_path(&ghosty_home.join("config.toml"))
                )),
                "{stdout}"
            );
            assert!(
                stdout.contains(&format!(
                    "settings: {}",
                    ghosty_config::quote_os_path(&ghosty_home.join("settings.toml"))
                )),
                "{stdout}"
            );
            assert!(
                stdout.contains("secret backend: file (inspection: metadata_only)"),
                "{stdout}"
            );
            assert!(
                stdout
                    .contains("legacy secret store: suppressed by explicit GHOSTY_HOME isolation"),
                "{stdout}"
            );
            assert!(!stdout.contains("synthetic-legacy-fixture"), "{stdout}");
        }

        assert_eq!(
            relative_paths(&sealed_home),
            before_paths,
            "dispatcher {args:?} must not create or migrate state below HOME"
        );
        assert_eq!(
            fs::read(&legacy).expect("read synthetic legacy store after diagnostic"),
            before_legacy,
            "dispatcher {args:?} must not rewrite the legacy store"
        );
        assert_eq!(
            fs::read(&legacy_settings).expect("read legacy settings after diagnostic"),
            legacy_settings_bytes,
            "dispatcher {args:?} must not rewrite legacy settings"
        );
        assert!(
            !primary_home.exists(),
            "dispatcher {args:?} must not create a primary Ghosty home or migrated state"
        );
        assert!(
            !ghosty_home.exists(),
            "dispatcher {args:?} must not create an explicit GHOSTY_HOME"
        );
    }
}

fn relative_paths(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_relative_paths(root, root, &mut paths);
    paths.sort();
    paths
}

fn collect_relative_paths(root: &Path, current: &Path, paths: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(current).expect("read synthetic state directory");
    for entry in entries {
        let entry = entry.expect("synthetic state directory entry");
        let path = entry.path();
        paths.push(
            path.strip_prefix(root)
                .expect("synthetic path below root")
                .to_path_buf(),
        );
        if entry.file_type().expect("synthetic entry type").is_dir() {
            collect_relative_paths(root, &path, paths);
        }
    }
}

fn ghosty_binary() -> PathBuf {
    if let Some(path) = option_env!("CARGO_BIN_EXE_ghosty") {
        return PathBuf::from(path);
    }
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_ghosty") {
        return PathBuf::from(path);
    }

    let mut path = std::env::current_exe().expect("current test executable path");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.push(format!("ghosty{}", std::env::consts::EXE_SUFFIX));
    path
}

/// A rustup shim may initialize its own toolchain state below `$HOME` when
/// `doctor` asks `rustc --version`. Preserve an already-configured toolchain
/// root so this test isolates Ghosty's own state contract.
fn preserve_host_rustup_home(command: &mut Command) {
    let rustup_home = std::env::var_os("RUSTUP_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".rustup"))
                .filter(|path| path.is_dir())
        });
    if let Some(rustup_home) = rustup_home {
        command.env("RUSTUP_HOME", rustup_home);
    }
}
