//! `/remote-env` opens the hosted Work launcher without taking source custody.

use std::path::Path;
use std::process::Stdio;

use crate::commands::traits::{CommandInfo, RegisterCommand};
use crate::dependencies::{ExternalTool, Git};
use crate::localization::{Locale, MessageId, tr};
use crate::tui::app::{App, AppAction};

use super::CommandResult;

const HOSTED_WORK_URL: &str = "https://app.ghosty.net/work";
const MAX_GIT_VALUE_BYTES: usize = 4 * 1024;

pub(in crate::commands) const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "remote-env",
    aliases: &[],
    usage: "/remote-env [open]",
    description_id: MessageId::CmdRemoteEnvDescription,
};

pub(in crate::commands) struct RemoteEnvCmd;

impl RegisterCommand for RemoteEnvCmd {
    fn info() -> &'static CommandInfo {
        &COMMAND_INFO
    }

    fn execute(app: &mut App, arg: Option<&str>) -> CommandResult {
        match arg.map(str::trim).filter(|value| !value.is_empty()) {
            None => CommandResult::message(remote_env_copy(
                app.ui_locale,
                MessageId::CmdRemoteEnvOverview,
            )),
            Some("open") => open_hosted_work(app),
            Some(_) => CommandResult::error(remote_env_copy(
                app.ui_locale,
                MessageId::CmdRemoteEnvSourceCustodyPolicy,
            )),
        }
    }
}

fn open_hosted_work(app: &App) -> CommandResult {
    let Some(target) = resolve_target(&app.workspace) else {
        return CommandResult::error(remote_env_copy(
            app.ui_locale,
            MessageId::CmdRemoteEnvUnavailable,
        ));
    };
    let url = hosted_work_url(&target.repo, &target.branch);
    let message = remote_env_copy(app.ui_locale, MessageId::CmdRemoteEnvOpening)
        .replace("{url}", &url)
        .replace("{repo}", &target.repo)
        .replace("{branch}", &target.branch);
    let label = tr(app.ui_locale, MessageId::CmdRemoteEnvBrowserLabel).into_owned();

    CommandResult::with_message_and_action(message, AppAction::OpenExternalUrl { url, label })
}

fn remote_env_copy(locale: Locale, id: MessageId) -> String {
    tr(locale, id)
        .replace("{command}", "/remote-env open")
        .replace("{origin}", "origin")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RemoteEnvTarget {
    repo: String,
    branch: String,
}

fn resolve_target(workspace: &Path) -> Option<RemoteEnvTarget> {
    let origin = read_git_value(
        workspace,
        &["config", "--local", "--get", "remote.origin.url"],
    )?;
    let repo = normalize_repo_slug(&origin)?;
    let branch = read_git_value(workspace, &["symbolic-ref", "--quiet", "--short", "HEAD"])?;
    if !valid_branch_name(&branch) {
        return None;
    }
    Some(RemoteEnvTarget { repo, branch })
}

fn read_git_value(workspace: &Path, args: &[&str]) -> Option<String> {
    let mut command = Git::command()?;
    let output = command.arg("-C").arg(workspace).args(args).output().ok()?;
    if !output.status.success()
        || output.stdout.is_empty()
        || output.stdout.len() > MAX_GIT_VALUE_BYTES
    {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    let value = value.trim_end_matches(&['\r', '\n'][..]);
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn valid_branch_name(branch: &str) -> bool {
    if branch.is_empty() || branch.len() > MAX_GIT_VALUE_BYTES {
        return false;
    }
    let Some(mut command) = Git::command() else {
        return false;
    };
    command
        .args(["check-ref-format", "--branch"])
        .arg(branch)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn hosted_work_url(repo: &str, branch: &str) -> String {
    format!(
        "{HOSTED_WORK_URL}?repo={}&branch={}",
        urlencoding::encode(repo),
        urlencoding::encode(branch),
    )
}

fn normalize_repo_slug(origin: &str) -> Option<String> {
    let origin = origin.trim();
    if origin.is_empty()
        || origin.len() > MAX_GIT_VALUE_BYTES
        || origin.chars().any(char::is_control)
    {
        return None;
    }

    let (host, path) = if starts_with_ascii_case(origin, "https://") {
        split_url_origin(&origin["https://".len()..], UrlScheme::Https)?
    } else if starts_with_ascii_case(origin, "ssh://") {
        split_url_origin(&origin["ssh://".len()..], UrlScheme::Ssh)?
    } else {
        split_scp_origin(origin)?
    };
    if !matches!(
        host.to_ascii_lowercase().as_str(),
        "github.com" | "cnb.cool"
    ) {
        return None;
    }
    normalize_repo_path(path)
}

#[derive(Debug, Clone, Copy)]
enum UrlScheme {
    Https,
    Ssh,
}

fn starts_with_ascii_case(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(prefix))
}

fn split_url_origin(origin: &str, scheme: UrlScheme) -> Option<(&str, &str)> {
    let (authority, path) = origin.split_once('/')?;
    if authority.is_empty() || path.is_empty() {
        return None;
    }
    let host_port = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    let host = match host_port.rsplit_once(':') {
        Some((host, port))
            if !host.is_empty()
                && !port.is_empty()
                && port.bytes().all(|byte| byte.is_ascii_digit())
                && (matches!(scheme, UrlScheme::Ssh) || port == "443") =>
        {
            host
        }
        Some(_) => return None,
        None => host_port,
    };
    (!host.is_empty()).then_some((host, path))
}

fn split_scp_origin(origin: &str) -> Option<(&str, &str)> {
    let (authority, path) = origin.split_once(':')?;
    let (_, host) = authority.rsplit_once('@')?;
    if host.is_empty() || path.is_empty() {
        return None;
    }
    Some((host, path))
}

fn normalize_repo_path(path: &str) -> Option<String> {
    if path.chars().any(|ch| matches!(ch, '?' | '#' | '\\')) {
        return None;
    }
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let mut parts = path.split('/');
    let namespace = parts.next()?;
    let repository = parts.next()?;
    if parts.next().is_some()
        || !valid_repo_component(namespace)
        || !valid_repo_component(repository)
    {
        return None;
    }
    Some(format!("{namespace}/{repository}"))
}

fn valid_repo_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && !matches!(value, "." | "..")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::localization::{Locale, MessageId, tr};
    use crate::tui::app::TuiOptions;
    use std::process::Command;
    use tempfile::TempDir;

    fn init_repo(origin: &str, branch: &str) -> TempDir {
        let temp = TempDir::new().expect("temp repository");
        let init = Command::new("git")
            .args(["init", "--quiet"])
            .arg(temp.path())
            .status()
            .expect("run git init");
        assert!(init.success());
        let set_origin = Command::new("git")
            .arg("-C")
            .arg(temp.path())
            .args(["config", "--local", "remote.origin.url", origin])
            .status()
            .expect("set origin");
        assert!(set_origin.success());
        let set_branch = Command::new("git")
            .arg("-C")
            .arg(temp.path())
            .args(["symbolic-ref", "HEAD"])
            .arg(format!("refs/heads/{branch}"))
            .status()
            .expect("set branch");
        assert!(set_branch.success());
        temp
    }

    fn app_for(workspace: &Path) -> App {
        let options = TuiOptions {
            ..crate::test_support::test_tui_options(workspace)
        };
        let mut app = crate::test_support::test_app_with_options(options);
        app.ui_locale = Locale::En;
        app
    }

    fn external_url(result: &CommandResult) -> &str {
        match result.action.as_ref() {
            Some(AppAction::OpenExternalUrl { url, .. }) => url,
            other => panic!("expected external URL action, got {other:?}"),
        }
    }

    #[test]
    fn bare_command_only_explains_the_source_boundary() {
        let temp = TempDir::new().expect("workspace");
        let mut app = app_for(temp.path());
        let result = RemoteEnvCmd::execute(&mut app, None);
        let message = result.message.expect("overview");
        assert!(!result.is_error);
        assert!(result.action.is_none());
        for boundary in ["unpushed", "dirty", "ignored", "secrets", "session state"] {
            assert!(message.contains(boundary), "missing boundary: {boundary}");
        }
    }

    #[test]
    fn open_builds_encoded_launcher_url_without_embedded_credentials() {
        let secret = "top-secret-token";
        let temp = init_repo(
            &format!("https://hunter:{secret}@github.com/blissito/ghostycode.git"),
            "feature/mobile&cloud-{url}",
        );
        let mut app = app_for(temp.path());
        let result = RemoteEnvCmd::execute(&mut app, Some("open"));
        assert!(!result.is_error);
        let url = external_url(&result);
        assert_eq!(
            url,
            "https://app.ghosty.net/work?repo=blissito%2Fghostycode&branch=feature%2Fmobile%26cloud-%7Burl%7D"
        );
        assert!(
            result
                .message
                .as_deref()
                .unwrap_or_default()
                .contains("feature/mobile&cloud-{url}")
        );
        assert!(!url.contains(secret));
        assert!(
            !result
                .message
                .as_deref()
                .unwrap_or_default()
                .contains(secret)
        );
    }

    #[test]
    fn supported_https_and_ssh_origins_normalize_to_namespace_and_repo() {
        for (origin, expected) in [
            (
                "https://github.com/blissito/ghostycode.git",
                "blissito/ghostycode",
            ),
            (
                "https://user:token@github.com/blissito/ghostycode",
                "blissito/ghostycode",
            ),
            (
                "ssh://git@github.com/blissito/ghostycode.git",
                "blissito/ghostycode",
            ),
            (
                "git@github.com:blissito/ghostycode.git",
                "blissito/ghostycode",
            ),
            ("https://cnb.cool/whale/ghosty.git", "whale/ghosty"),
            ("ssh://git@cnb.cool:2222/whale/ghosty.git", "whale/ghosty"),
            ("git@cnb.cool:whale/ghosty.git", "whale/ghosty"),
        ] {
            assert_eq!(
                normalize_repo_slug(origin).as_deref(),
                Some(expected),
                "{origin}"
            );
        }
    }

    #[test]
    fn unsupported_or_ambiguous_origins_are_rejected_without_echoing_them() {
        let secret = "do-not-echo-this-token";
        let temp = init_repo(
            &format!("https://user:{secret}@gitlab.com/acme/widgets.git"),
            "main",
        );
        let mut app = app_for(temp.path());
        let result = RemoteEnvCmd::execute(&mut app, Some("open"));
        assert!(result.is_error);
        assert!(result.action.is_none());
        assert!(
            !result
                .message
                .as_deref()
                .unwrap_or_default()
                .contains(secret)
        );

        for origin in [
            "http://github.com/acme/widgets.git",
            "git://github.com/acme/widgets.git",
            "https://github.com/acme/widgets/extra.git",
            "https://github.com/acme/widgets.git?token=secret",
            "file:///tmp/widgets.git",
            "/tmp/widgets.git",
        ] {
            assert_eq!(normalize_repo_slug(origin), None, "{origin}");
        }
    }

    #[test]
    fn upload_migrate_sync_and_unknown_operations_are_read_only_rejections() {
        let temp = TempDir::new().expect("workspace");
        let mut app = app_for(temp.path());
        for operation in ["upload", "migrate", "sync", "surprise"] {
            let result = RemoteEnvCmd::execute(&mut app, Some(operation));
            assert!(result.is_error, "{operation}");
            assert!(result.action.is_none(), "{operation}");
            let message = result.message.expect("policy message");
            assert!(message.contains("does not upload, migrate, or sync"));
            assert!(message.contains("/remote-env open"));
        }
    }

    #[test]
    fn opening_requires_a_symbolic_branch() {
        let temp = init_repo("git@github.com:acme/widgets.git", "main");
        let identity = Command::new("git")
            .arg("-C")
            .arg(temp.path())
            .args(["config", "user.name", "Ghosty Test"])
            .status()
            .expect("set test name");
        assert!(identity.success());
        let identity = Command::new("git")
            .arg("-C")
            .arg(temp.path())
            .args(["config", "user.email", "test@ghosty.invalid"])
            .status()
            .expect("set test email");
        assert!(identity.success());
        let commit = Command::new("git")
            .arg("-C")
            .arg(temp.path())
            .args(["commit", "--allow-empty", "--quiet", "-m", "fixture"])
            .status()
            .expect("create fixture commit");
        assert!(commit.success());
        let detach = Command::new("git")
            .arg("-C")
            .arg(temp.path())
            .args(["checkout", "--detach", "--quiet", "HEAD"])
            .status()
            .expect("detach HEAD");
        assert!(detach.success());

        let mut app = app_for(temp.path());
        let result = RemoteEnvCmd::execute(&mut app, Some("open"));
        assert!(result.is_error);
        assert!(result.action.is_none());
    }

    #[test]
    fn localized_copy_preserves_composed_placeholders() {
        let cases: &[(MessageId, &[&str])] = &[
            (MessageId::CmdRemoteEnvOverview, &["{command}"]),
            (
                MessageId::CmdRemoteEnvOpening,
                &["{repo}", "{branch}", "{origin}", "{url}"],
            ),
            (
                MessageId::CmdRemoteEnvUnavailable,
                &["{origin}", "{command}"],
            ),
            (MessageId::CmdRemoteEnvSourceCustodyPolicy, &["{command}"]),
        ];
        for locale in Locale::shipped_complete() {
            for (id, placeholders) in cases {
                let message = tr(*locale, *id);
                for placeholder in *placeholders {
                    assert!(
                        message.contains(placeholder),
                        "{} {id:?} lost {placeholder}",
                        locale.tag()
                    );
                }
            }
        }
    }
}
