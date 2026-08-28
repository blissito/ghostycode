# winget packaging for GhostyCode

This directory holds the source winget manifest for `blissito.GhostyCode` (resolves #1561).
Winget installs the single runtime under `ghosty` + `ghosty-tui`; it never
installs a `ghosty-tui` command. GitHub Releases retain byte-identical
`ghosty-tui-*` filenames only for legacy updater compatibility.

## Files

- `blissito.GhostyCode.yaml` — singleton manifest for `winget install blissito.GhostyCode`. The
  installers all point at the signed (or checksum-verified) GitHub Release assets for the same
  version (`GhostyCodeSetup.exe` for x64 NSIS, plus portable ZIP fallbacks for x64/arm64).
- `generate-winget-manifest.sh` — bumps `PackageVersion`, `ReleaseDate`, and the four
  `InstallerSha256` placeholders from a local `release-assets/` checkout.
- `.winget/blissito.GhostyCode.yaml` (repo root) is a verbatim mirror for tooling that expects `.winget/`.
  Keep both in sync; `packaging/winget/blissito.GhostyCode.yaml` is canonical.

## Version flow

1. Tag `vX.Y.Z` publishes `GhostyCodeSetup.exe`, `ghosty-windows-x64.zip`,
   `ghosty-windows-x64-portable.zip`, `ghosty-windows-arm64.zip`,
   `ghosty-windows-arm64-portable.zip`, and `ghosty-artifacts-sha256.txt`.
2. From the release tag checkout, run:
   ```bash
   ./packaging/winget/generate-winget-manifest.sh X.Y.Z /path/to/release-assets
   ```
   It rewrites both `packaging/winget/blissito.GhostyCode.yaml` and `.winget/blissito.GhostyCode.yaml`
   with the fresh version and the four SHA-256 values extracted from `ghosty-artifacts-sha256.txt`.
3. Validate locally with `winget validate` (requires winget + the manifest schema):
   ```bash
   winget validate --manifest packaging/winget/blissito.GhostyCode.yaml
   # or the Microsoft validator in winget-pkgs CI:
   # https://github.com/microsoft/winget-pkgs#validation
   ```
4. Submit to [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs) via
   `wingetcreate` or a manual PR that adds `manifests/h/blissito/ghostycode/X.Y.Z/`:
   ```bash
   wingetcreate update blissito.GhostyCode --version X.Y.Z --urls \
     https://github.com/blissito/ghostycode/releases/download/vX.Y.Z/GhostyCodeSetup.exe \
     https://github.com/blissito/ghostycode/releases/download/vX.Y.Z/ghosty-windows-x64.zip \
     https://github.com/blissito/ghostycode/releases/download/vX.Y.Z/ghosty-windows-x64-portable.zip \
     https://github.com/blissito/ghostycode/releases/download/vX.Y.Z/ghosty-windows-arm64.zip \
     https://github.com/blissito/ghostycode/releases/download/vX.Y.Z/ghosty-windows-arm64-portable.zip
   ```
   The generated PR must pass the winget-pkgs validation workflow before merge.

## Single-binary note

Until v0.9.4 the release matrix installed three commands (`ghosty`, `ghosty-tui`,
and `ghosty-tui`). Since v0.9.5 each target installs only the byte-identical
`ghosty` + `ghosty-tui` commands (Windows also ships `ghosty.bat`). GitHub
Releases retain `ghosty-tui-*` compatibility filenames for old updater
clients, but the winget ZIP `NestedInstallerFiles` lists only the two current
PATH commands; `ghosty-tui.exe` is intentionally absent.

## FreeBSD

FreeBSD has no prebuilt GitHub Release asset (see `docs/INSTALL.md` § FreeBSD). Install via Cargo:

```bash
pkg install -y rust pkgconf  # or ports-mgmt/pkg
cargo install ghosty-cli --locked   # provides `ghosty`
```

The npm wrapper on FreeBSD exits with `Unsupported platform: freebsd` and points to the Cargo path.
A native `pkg install ghosty` port is tracked as a follow-up to #1097 — contributions welcome
under `packaging/freebsd/`.
