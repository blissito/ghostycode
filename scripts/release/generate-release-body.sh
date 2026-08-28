#!/usr/bin/env bash
# Generate the GitHub Release body for a tag.
#
# Usage: generate-release-body.sh <vX.Y.Z> [path/to/CHANGELOG.md]
#
# The install/verify sections are static; the release notes and contributor
# credits come from the CHANGELOG section for the version, so they can never
# drift the way a hand-edited workflow body does.
set -euo pipefail

tag="${1:?usage: $0 <vX.Y.Z> [CHANGELOG.md]}"
changelog="${2:-CHANGELOG.md}"
version="${tag#v}"

section="$(awk -v version="${version}" '
  index($0, "## [" version "]") == 1 { in_section = 1; next }
  in_section && /^## \[/ { exit }
  in_section { print }
' "${changelog}")"

contributors="$(printf '%s\n' "${section}" | awk '
  /^### Contributors[[:space:]]*$/ { in_contributors = 1; next }
  in_contributors && /^### / { exit }
  in_contributors { print }
')"

notes="$(printf '%s\n' "${section}" | awk '
  /^### Contributors[[:space:]]*$/ { in_contributors = 1; next }
  in_contributors && /^### / { in_contributors = 0 }
  !in_contributors { print }
')"

cat <<EOF
> **Ghosty** is the public product from Shannon Labs. The \`ghosty\`
> command, npm package, and release-asset names remain lowercase technical
> identifiers. The legacy npm package \`deepseek-tui\` is deprecated and
> receives no further releases. Users coming from v0.8.x legacy \`deepseek\` /
> \`deepseek-tui\` names should migrate with \`docs/REBRAND.md\`.

## Install

### Recommended — npm (one command, both entrypoints)

\`\`\`bash
npm install -g ghosty
\`\`\`

The wrapper downloads the matched \`ghosty\` and \`ghosty-tui\` command assets
from this Release. Both contain the same compiled runtime.

### Docker / GHCR

\`\`\`bash
docker run --rm -it \\
  -e DEEPSEEK_API_KEY="\$DEEPSEEK_API_KEY" \\
  -v ghosty-home:/home/ghosty/.ghosty \\
  ghcr.io/blissito/ghostycode:${tag}
\`\`\`

The image exposes the same runtime as both \`ghosty\` and \`ghosty-tui\`. The
\`latest\` tag is also updated on release.

### Cargo (Linux / macOS)

\`\`\`bash
cargo install ghosty-cli --locked
\`\`\`

The Cargo package installs \`ghosty\`. Cargo cannot create a second command
alias from one binary target; users who want the shorter spelling can add a
\`ghosty-tui\` symlink to that installed executable. The npm, Homebrew, archive,
shell-installer, and container channels install both command names directly.

### Manual download — platform archives (recommended)

Each archive below contains the same runtime under the \`ghosty\` and
\`ghosty-tui\` command names, plus an install script:

| Platform | Archive | Install script |
|---|---|---|
| Linux x64 | \`ghosty-linux-x64.tar.gz\` | \`install.sh\` |
| Linux ARM64 | \`ghosty-linux-arm64.tar.gz\` | \`install.sh\` |
| Android ARM64 (Termux) | \`ghosty-android-arm64.tar.gz\` | \`install.sh\` |
| macOS x64 | \`ghosty-macos-x64.tar.gz\` | \`install.sh\` |
| macOS ARM | \`ghosty-macos-arm64.tar.gz\` | \`install.sh\` |
| Windows x64 (installer) | \`GhostyCodeSetup.exe\` | NSIS setup |
| Windows x64 | \`ghosty-windows-x64.zip\` | \`install.bat\` |
| Windows x64 (portable) | \`ghosty-windows-x64-portable.zip\` | — |
| Windows ARM64 | \`ghosty-windows-arm64.zip\` | \`install.bat\` |
| Windows ARM64 (portable) | \`ghosty-windows-arm64-portable.zip\` | — |

**Unix (Linux / macOS):**
\`\`\`bash
tar xzf ghosty-<platform>.tar.gz
cd ghosty-<platform>
./install.sh
\`\`\`

**Windows:**
- For the installer path, run \`GhostyCodeSetup.exe\`; it installs
  \`ghosty.exe\`, \`ghosty-tui.exe\`, and \`ghosty.bat\` under
  \`%LOCALAPPDATA%\\Programs\\GhostyCode\\bin\`, adds that directory to the
  current-user PATH, and creates a Start Menu shortcut that prefers
  Windows Terminal (\`wt.exe\`) when it is installed.
- Extract the archive for your machine: \`ghosty-windows-x64.zip\` or
  \`ghosty-windows-arm64.zip\`
- Double-click \`ghosty.bat\` (not the raw \`.exe\`) to launch
- Run \`install.bat\` to copy the binaries and launcher to \`%USERPROFILE%\\bin\`
- Add \`%USERPROFILE%\\bin\` to your PATH

The **portable** Windows archive skips the install script — extract and run \`ghosty.bat\` from any directory. The NSIS installer is currently unsigned and may trigger Windows SmartScreen until a signing certificate is wired into the release pipeline.

Each platform also has **bare, unarchived** \`ghosty-<platform>\` and
\`ghosty-tui-<platform>\` assets. The seven \`ghosty-tui-<platform>\` filenames
attached to v0.9.5 are byte-identical compatibility copies used only to let
already-installed v0.9.4 clients discover and cross this single-binary
transition; current installers do not expose a third runtime. The legacy npm
package \`deepseek-tui\` is deprecated and is not republished. For migration
from v0.8.x legacy binary names, see \`docs/REBRAND.md\`.

### Verify (recommended)

Download the checksum manifests from this Release and verify:

\`\`\`bash
# Linux — archive bundles
sha256sum -c ghosty-bundles-sha256.txt --ignore-missing

# Linux — individual binaries
sha256sum -c ghosty-artifacts-sha256.txt --ignore-missing

# macOS
shasum -a 256 -c ghosty-bundles-sha256.txt --ignore-missing
shasum -a 256 -c ghosty-artifacts-sha256.txt --ignore-missing
\`\`\`

## What's in ${tag}
EOF

if [[ -n "${notes}" ]]; then
  printf '%s\n' "${notes}"
else
  printf '%s\n' "See the changelog link below for this release's notes."
fi

cat <<EOF

## Contributors
EOF

if [[ -n "${contributors}" ]]; then
  printf '%s\n' "${contributors}"
else
  printf '%s\n' "Thank you to everyone whose reports, PRs, reviews, and reproductions shaped this release."
fi

cat <<EOF

See [CHANGELOG.md](https://github.com/blissito/ghostycode/blob/main/CHANGELOG.md) for full notes and [docs/CHANGELOG_ARCHIVE.md](https://github.com/blissito/ghostycode/blob/main/docs/CHANGELOG_ARCHIVE.md) for older releases.
EOF
