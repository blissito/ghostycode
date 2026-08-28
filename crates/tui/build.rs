use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    ghosty_build_support::declare_rerun_conditions(&manifest_dir);
    configure_windows_stack();
    ghosty_build_support::emit_build_version(&manifest_dir, env!("CARGO_PKG_VERSION"));
}

fn configure_windows_stack() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    match std::env::var("CARGO_CFG_TARGET_ENV").as_deref() {
        Ok("msvc") => {
            println!("cargo:rustc-link-arg-bin=ghosty-tui=/STACK:8388608");
        }
        Ok("gnu") => {
            println!("cargo:rustc-link-arg-bin=ghosty-tui=-Wl,--stack,8388608");
        }
        _ => {}
    }
}
