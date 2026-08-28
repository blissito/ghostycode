#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> std::process::ExitCode {
    // Reset SIGPIPE to SIG_DFL so piping ghosty output into a command that
    // exits early (e.g. `ghosty doctor | head`) terminates the process
    // cleanly with exit code 141 instead of panicking on the broken-pipe
    // write. Many execution environments (systemd, Docker, some shells)
    // inherit SIGPIPE set to SIG_IGN, which makes write(2) return EPIPE;
    // Rust's `println!` then treats that io::Error as fatal and panics.
    // See issue #4030.
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    // Single-binary argv0 dispatch: `ghosty-tui` is now an alias for `ghosty`
    // without a second compiled artifact. Checking the binary basename keeps
    // the install surface at one file while preserving the six-keystroke save.
    let _ = std::env::args().next().and_then(|argv0| {
        let base = std::path::Path::new(&argv0)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let trimmed = base
            .strip_suffix(std::env::consts::EXE_SUFFIX)
            .unwrap_or(base);
        if trimmed == "ghosty-tui" {
            // No-op: the single `ghosty` binary handles both names.
        }
        None::<()>
    });

    ghosty_cli::run_cli()
}
