use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use ghosty_config::{
    CONFIG_FILE_NAME, GHOSTY_APP_DIR, LEGACY_APP_DIR, default_config_path, ghosty_home,
};
use ghosty_secrets::FileKeyringStore;
use ghosty_state::StateStore;

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct ProcessEnv {
    _lock: MutexGuard<'static, ()>,
    cwd: PathBuf,
    home: Option<OsString>,
    userprofile: Option<OsString>,
    ghosty_home: Option<OsString>,
}

impl ProcessEnv {
    fn install(root: &Path, home: &Path, userprofile: &Path, ghosty_home: &OsString) -> Self {
        let lock = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let prior = Self {
            _lock: lock,
            cwd: std::env::current_dir().expect("current directory"),
            home: std::env::var_os("HOME"),
            userprofile: std::env::var_os("USERPROFILE"),
            ghosty_home: std::env::var_os("GHOSTY_HOME"),
        };

        // SAFETY: this integration-test process serializes all environment and
        // current-directory mutation with ENV_LOCK.
        unsafe {
            std::env::set_var("HOME", home);
            std::env::set_var("USERPROFILE", userprofile);
            std::env::set_var("GHOSTY_HOME", ghosty_home);
        }
        std::env::set_current_dir(root).expect("install isolated current directory");
        prior
    }
}

impl Drop for ProcessEnv {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.cwd).expect("restore current directory");
        // SAFETY: this integration-test process serializes all environment and
        // current-directory mutation with ENV_LOCK.
        unsafe {
            restore_var("HOME", self.home.take());
            restore_var("USERPROFILE", self.userprofile.take());
            restore_var("GHOSTY_HOME", self.ghosty_home.take());
        }
    }
}

unsafe fn restore_var(name: &str, value: Option<OsString>) {
    match value {
        Some(value) => unsafe { std::env::set_var(name, value) },
        None => unsafe { std::env::remove_var(name) },
    }
}

#[test]
fn whitespace_override_uses_home_before_userprofile_and_allows_legacy_fallback() {
    let tmp = tempfile::tempdir().expect("temporary root");
    let home = tmp.path().join("home");
    let userprofile = tmp.path().join("userprofile");
    let legacy = home.join(LEGACY_APP_DIR);
    std::fs::create_dir_all(&legacy).expect("legacy directory");
    std::fs::write(legacy.join(CONFIG_FILE_NAME), b"provider = \"ollama\"\n")
        .expect("legacy config");
    std::fs::write(legacy.join("state.db"), b"").expect("legacy state marker");

    let _env = ProcessEnv::install(tmp.path(), &home, &userprofile, &OsString::from(" \t "));

    assert_eq!(
        ghosty_home().expect("config home"),
        home.join(GHOSTY_APP_DIR)
    );
    assert_eq!(
        default_config_path().expect("config path"),
        legacy.join(CONFIG_FILE_NAME)
    );

    let state = StateStore::open(None).expect("default state store");
    assert_eq!(state.db_path(), legacy.join("state.db"));

    let (primary_secrets, legacy_secrets) =
        FileKeyringStore::default_paths_read_only().expect("secret paths");
    assert_eq!(
        primary_secrets,
        home.join(GHOSTY_APP_DIR)
            .join("secrets")
            .join("secrets.json")
    );
    assert_eq!(
        legacy_secrets,
        Some(legacy.join("secrets").join("secrets.json"))
    );
}

#[cfg(unix)]
#[test]
fn non_unicode_override_is_one_explicit_isolation_boundary() {
    use std::os::unix::ffi::OsStringExt;

    use ghosty_state::default_state_db_path;

    let tmp = tempfile::tempdir().expect("temporary root");
    let home = tmp.path().join("home");
    let userprofile = tmp.path().join("userprofile");
    let explicit = tmp
        .path()
        .join(OsString::from_vec(b"ghosty-\xff-home".to_vec()));
    let _env = ProcessEnv::install(
        tmp.path(),
        &home,
        &userprofile,
        &explicit.as_os_str().to_os_string(),
    );

    assert_eq!(ghosty_home().expect("config home"), explicit);
    assert_eq!(
        default_config_path().expect("config path"),
        explicit.join(CONFIG_FILE_NAME)
    );

    assert_eq!(default_state_db_path(), explicit.join("state.db"));

    let (primary_secrets, legacy_secrets) =
        FileKeyringStore::default_paths_read_only().expect("secret paths");
    assert_eq!(
        primary_secrets,
        explicit.join("secrets").join("secrets.json")
    );
    assert_eq!(legacy_secrets, None);
}
