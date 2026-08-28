# Docker

Ghosty publishes a multi-arch Linux image to GitHub Container Registry
for each release.

```bash
docker pull ghcr.io/blissito/ghostycode:latest
```

## Quick start

Run the published image with a Docker-managed data volume:

```bash
docker volume create ghosty-home

docker run --rm -it \
  -e DEEPSEEK_API_KEY="$DEEPSEEK_API_KEY" \
  -v ghosty-home:/home/ghosty/.ghosty \
  -v "$PWD:/workspace" \
  -w /workspace \
  ghcr.io/blissito/ghostycode:latest
```

Use a pinned release tag for reproducible installs:

```bash
docker run --rm -it \
  -e DEEPSEEK_API_KEY="$DEEPSEEK_API_KEY" \
  -v ghosty-home:/home/ghosty/.ghosty \
  -v "$PWD:/workspace" \
  -w /workspace \
  ghcr.io/blissito/ghostycode:vX.Y.Z
```

Replace `vX.Y.Z` with a tag from
[GitHub Releases](https://github.com/blissito/ghostycode/releases).

## Default image contract

`ghcr.io/blissito/ghostycode:latest` and the semver tags are conservative runtime
images:

- the container runs as the non-root `ghosty` user with UID/GID `1000:1000`
- the image does not grant passwordless `sudo`
- the image is meant to run Ghosty against mounted workspaces, not to mutate
  the base operating system at runtime
- user state belongs in a volume mounted at `/home/ghosty/.ghosty`

That default is intentional. Keep using it for the smallest trust boundary. If a
project needs `apt-get`, compiler toolchains, Node/Python package managers,
custom CA certificates, or other host-like setup inside Docker, build an
explicit toolbox image instead of changing the default image contract.

## Opt-in toolbox/custom image

The repository includes an example
[`docs/examples/Dockerfile.toolbox`](examples/Dockerfile.toolbox) that extends
the official image with passwordless `sudo` and common development packages.
Build it with a pinned Ghosty tag when you want repeatable project
environments:

```bash
docker build -f docs/examples/Dockerfile.toolbox \
  --build-arg GHOSTY_IMAGE=ghcr.io/blissito/ghostycode:vX.Y.Z \
  --build-arg TOOLBOX_PACKAGES="git openssh-client curl build-essential pkg-config python3 python3-pip nodejs npm" \
  -t ghosty-toolbox:my-project .
```

Use `latest` only for throwaway testing. For shared projects, keep the
`GHOSTY_IMAGE` value pinned and review package additions like any other
development-environment change.

Run the toolbox image with the same workspace and state mounts:

```bash
docker volume create ghosty-my-project-home

docker run --rm -it \
  -e DEEPSEEK_API_KEY="$DEEPSEEK_API_KEY" \
  -v ghosty-my-project-home:/home/ghosty/.ghosty \
  -v "$PWD:/workspace" \
  -w /workspace \
  ghosty-toolbox:my-project
```

Inside this opt-in image, Ghosty can use commands such as
`sudo apt-get update` and `sudo apt-get install -y <package>`. For repeatable
containers, prefer baking those packages into the toolbox Dockerfile instead of
letting a long-lived container drift.

Do not bake API keys, SSH private keys, or other secrets into custom images.
Pass API keys at runtime and mount any SSH material deliberately, preferably
read-only and only for projects that need it.

### Compose toolbox template

If you prefer a repeatable `docker compose` entry point, use
[`docs/examples/compose.toolbox.yml`](examples/compose.toolbox.yml). It builds
the toolbox image from [`docs/examples/Dockerfile.toolbox`](examples/Dockerfile.toolbox)
and keeps the project state volume explicit:

```bash
GHOSTY_IMAGE=ghcr.io/blissito/ghostycode:vX.Y.Z \
GHOSTY_TOOLBOX_IMAGE=ghosty-toolbox:my-project \
GHOSTY_HOME_VOLUME=ghosty-my-project-home \
GHOSTY_WORKSPACE="$PWD" \
docker compose -f docs/examples/compose.toolbox.yml run --rm ghosty
```

Use a different `GHOSTY_TOOLBOX_IMAGE` and `GHOSTY_HOME_VOLUME` for each
project that needs an independent toolchain or independent `.ghosty` state.
The Compose file also shows opt-in, read-only mounts for SSH material and local
CA certificates; keep those commented out unless the project needs them.

## Multiple independent projects

Use one named state volume per project so sessions, config, skills, memory, and
the offline queue do not bleed across workspaces:

```bash
project="$(basename "$PWD")"
image="ghosty-toolbox:${project}"
docker volume create "ghosty-${project}-home"

docker run --rm -it \
  --name "ghosty-${project}" \
  -e DEEPSEEK_API_KEY="$DEEPSEEK_API_KEY" \
  -v "ghosty-${project}-home:/home/ghosty/.ghosty" \
  -v "$PWD:/workspace" \
  -w /workspace \
  "$image"
```

For projects with different toolchains, build different toolbox tags, for
example `ghosty-toolbox:frontend` and `ghosty-toolbox:backend`. The
separate launcher idea discussed in issue #2217 can build on this contract, but
it is intentionally outside the core Docker image.

## Project bootstrap scripts

Ghosty does not automatically execute `.ghosty/setup.sh` or legacy
`.deepseek/setup.sh`. If you keep one of those files as a local project recipe,
run it explicitly. For shared team setup, prefer a committed project script or
the toolbox Dockerfile so the environment can be reviewed and rebuilt.

For example, to run a committed bootstrap script before starting Ghosty:

```bash
docker run --rm -it \
  -e DEEPSEEK_API_KEY="$DEEPSEEK_API_KEY" \
  -v ghosty-my-project-home:/home/ghosty/.ghosty \
  -v "$PWD:/workspace" \
  -w /workspace \
  --entrypoint bash \
  ghosty-toolbox:my-project \
  -lc './scripts/bootstrap-dev.sh && exec ghosty'
```

Use the toolbox image for bootstrap scripts that need `sudo`. The default image
will not elevate privileges.

## Custom CA certificates and proxies

For corporate proxies, dev-sidecar, or self-signed internal services, prefer
baking trusted CA certificates into a custom toolbox image:

```dockerfile
USER root
COPY docker/certs/*.crt /usr/local/share/ca-certificates/
RUN update-ca-certificates
USER ghosty
```

All files copied into `/usr/local/share/ca-certificates/` must use the `.crt`
extension. Keep private CA material out of public images.

For a local-only run, mount certificates read-only and update the trust store at
container start:

```bash
docker run --rm -it \
  -e DEEPSEEK_API_KEY="$DEEPSEEK_API_KEY" \
  -v ghosty-my-project-home:/home/ghosty/.ghosty \
  -v "$PWD:/workspace" \
  -v "$PWD/docker/certs:/usr/local/share/ca-certificates/local:ro" \
  -w /workspace \
  --entrypoint bash \
  ghosty-toolbox:my-project \
  -lc 'sudo update-ca-certificates && exec ghosty'
```

This CA workflow requires the opt-in toolbox image because the default image
does not include passwordless `sudo`.

## Local build

Build the image locally from a checkout:

```bash
docker build -t ghosty .
```

Then run it with the same Docker-managed data volume:

```bash
docker run --rm -it \
  -e DEEPSEEK_API_KEY="$DEEPSEEK_API_KEY" \
  -v ghosty-home:/home/ghosty/.ghosty \
  -v "$PWD:/workspace" \
  -w /workspace \
  ghosty
```

Docker Hub publishing is not configured; GHCR is the supported prebuilt image
registry.

## Environment variables

| Variable              | Required | Description                                      |
|-----------------------|----------|--------------------------------------------------|
| `DEEPSEEK_API_KEY`    | yes      | DeepSeek API key                                 |
| `DEEPSEEK_BASE_URL`   | no       | Custom API base URL (e.g. `https://api.deepseek.com`) |
| `DEEPSEEK_NO_COLOR`   | no       | Set to `1` to disable terminal colour output     |

## Volumes

Mount `/home/ghosty/.ghosty` to persist sessions, config, skills, memory,
and the offline queue across container restarts. The image also keeps
`/home/ghosty/.deepseek` available for legacy compatibility. A
Docker-managed named volume is the safest default because Docker creates it with
ownership the container can write:

```bash
-v ghosty-home:/home/ghosty/.ghosty
```

Without this mount the container starts fresh each time.

If you bind-mount an existing host directory instead, the image runs as the
non-root `ghosty` user with UID/GID `1000:1000`. The mounted directory must be
writable by that user, or startup can fail while creating runtime directories
under `.ghosty/tasks`. On Linux hosts, either use the named volume above or
prepare the bind mount explicitly:

```bash
mkdir -p ~/.ghosty
sudo chown -R 1000:1000 ~/.ghosty

docker run --rm -it \
  -e DEEPSEEK_API_KEY="$DEEPSEEK_API_KEY" \
  -v ~/.ghosty:/home/ghosty/.ghosty \
  ghcr.io/blissito/ghostycode:latest
```

That `chown` changes ownership of the host `~/.ghosty` directory. Skip it if
you do not want the container UID to own your local config, and use a named
volume instead.

## Non-interactive / pipeline usage

When stdin is not a TTY, `ghosty` drops to the dispatcher's one-shot mode
(`ghosty -c "…"`). Pipe a prompt on stdin:

```bash
echo "Explain the Cargo.toml in structured English." | \
  docker run --rm -i -e DEEPSEEK_API_KEY ghcr.io/blissito/ghostycode:latest
```

## Building locally

```bash
# Single platform (your host architecture)
docker build -t ghosty .

# Multi-platform (requires a builder with emulation)
docker buildx create --use
docker buildx build --platform linux/amd64,linux/arm64 -t ghosty .
```

## Devcontainer

The repository includes a [`.devcontainer/devcontainer.json`](../.devcontainer/devcontainer.json)
configuration for VS Code / GitHub Codespaces. It builds a dedicated development
image with the Rust toolchain, Git, `pkg-config`, and the DBus development headers
required by the workspace. The first open runs `cargo build --locked` and installs
rust-analyzer and the other editor extensions.

The source checkout remains mounted from the host. GhostyCode state and Cargo build
artifacts use Docker named volumes instead, so the configuration works when VS Code
cannot provide a POSIX-style `HOME` variable (notably on Windows), and builds do not
write thousands of small files through a Windows bind mount. Rebuild the container
after changing the Dev Container configuration.

## Release status

Docker image publishing is part of the release gate. The image is published to
GHCR for `linux/amd64` and `linux/arm64` with semver tags plus `latest`.
