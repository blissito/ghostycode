# FreeBSD packaging for GhostyCode

Resolves #1097 — FreeBSD install via `cargo` today, native `pkg` port planned.

## Current: cargo install (supported)

FreeBSD has no prebuilt GitHub Release asset. The npm wrapper intentionally fails with
`Unsupported platform: freebsd` and points to the Cargo path. Build from source:

```bash
pkg install -y rust pkgconf git  # or: pkg install -y lang/rust devel/pkgconf devel/git
cargo install ghosty-cli --locked   # provides `ghosty`
ghosty --version
ghosty doctor
```

Release installers (v0.9.5+) expose only `ghosty` + `ghosty-tui`. Cargo installs
`ghosty`; users who want the optional short name can add a `ghosty-tui` symlink.
There is no separate `ghosty-tui` binary to install. If you previously
installed `ghosty-tui` from ports or Cargo, run
`cargo uninstall ghosty-tui`, then rebuild `ghosty-cli`.

Linux `libdbus-1` / `pkg-config` build deps are not needed on FreeBSD for the default
feature set; the `rquickjs` FreeBSD bindings are generated at build time via `bindgen`
(see `crates/tui/build.rs` and the `1582ba965`/`5eb0385e8` FreeBSD bindgen fix).

## Target validation

- Rust tier-2 target `x86_64-unknown-freebsd` is validated in CI via `cargo check --target x86_64-unknown-freebsd -p ghosty-cli --locked` on the `release/0.9.5` branch.
- `aarch64-unknown-freebsd` cross-check is tracked for future hardware.
- The release matrix in `.github/workflows/release-artifacts.yml` remains 7×1 (Linux musl x64, Linux gnu arm64, Android arm64, macOS x64/arm64, Windows x64/arm64). FreeBSD is a source-build target — no prebuilt asset, no matrix bloat, but the row in `docs/INSTALL.md` is authoritative.

## Future: native pkg / port

A proper FreeBSD port (`ports-mgmt` / `pkg install ghosty`) is the long-game follow-up to #1097.
Desired shape (not yet submitted):

```
ports/sysutils/ghosty/
  Makefile     # USES=cargo, CARGO_CRATES via `cargo make-port` or `cargo-crates` helper
  distinfo
  pkg-descr
  pkg-plist   # bin/ghosty, bin/ghosty-tui
```

The Makefile should `BUILD_DEPENDS` on `lang/rust` and `devel/pkgconf`, run `cargo build --locked --release -p ghosty-cli`,
and install only `ghosty` + `ghosty-tui`. Tests can reuse `cargo test -p ghosty-cli --lib` if desired.

If you can test on FreeBSD 14.x (amd64), please run the smoke from `docs/INSTALL.md#7-build-from-source`
and report the `ghosty doctor` JSON in #1097.

## Verification

```bash
# on a FreeBSD 14.1 amd64 host or VM
rustc --version  # >= 1.88
cargo install --path crates/cli --locked
ghosty --version
ghosty exec --auto "run pwd"
```
